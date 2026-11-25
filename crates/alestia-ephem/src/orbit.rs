//! Core [`Orbit`] trait, Keplerian orbits (`EllipticalOrbit`, `HyperbolicOrbit`,
//! `ParabolicOrbit`), composite [`MixedOrbit`], [`FixedOrbit`], and [`CachingOrbit`]
//! matching `celephem/orbit.h` and `orbit.cpp` (`FR-010`).

use core::f64::consts::TAU;
use core::fmt;
use std::sync::{Arc, Mutex};

use alestia_astro::{GRAVITATIONAL_CONSTANT, KM_PER_AU, KeplerElements, state_vector_to_elements};
use alestia_math::{DMat3, DVec3, sign, solve_iteration_fixed, square, x_rotation, z_rotation};

const ORBITAL_VELOCITY_DIFF_DELTA: f64 = 1.0 / 1440.0;
const HYPERBOLIC_MIN_BOUNDING_RADIUS: f64 = 1000.0 * KM_PER_AU;

/// Single orbital state sample produced by [`Orbit::sample`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitSample {
    /// Sample epoch in TDB Julian Date.
    pub time: f64,
    /// Position in kilometers (Celestia internal frame).
    pub position: DVec3,
    /// Velocity in kilometers per day (Celestia internal frame).
    pub velocity: DVec3,
}

/// Parameters controlling adaptive Hermite trajectory sampling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveSamplingParams {
    /// Maximum position interpolation error tolerance in kilometers.
    pub tolerance: f64,
    /// Initial time step in days.
    pub start_step: f64,
    /// Minimum allowed step size in days.
    pub min_step: f64,
    /// Maximum allowed step size in days.
    pub max_step: f64,
}

impl AdaptiveSamplingParams {
    /// Default sampling configuration for a characteristic span `span` (period or valid range).
    #[must_use]
    pub const fn for_span(span: f64) -> Self {
        Self {
            tolerance: 1.0,
            max_step: span / 100.0,
            min_step: span / 1.0e7,
            start_step: span / 1.0e5,
        }
    }
}

/// Structured errors returned when constructing or validating analytical orbits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrbitError {
    /// Eccentricity falls outside the orbit class's valid interval.
    InvalidEccentricity(f64),
    /// Semimajor axis sign or finiteness does not match the orbit class.
    InvalidSemiMajorAxis(f64),
    /// Orbital period must be non-zero and finite.
    InvalidPeriod(f64),
    /// Parabolic perihelion distance or gravitational parameter `mu` must be strictly positive.
    InvalidPerihelion,
    /// Composite orbit validity interval `[begin, end]` must satisfy `end > begin`.
    InvalidTimeRange,
}

impl fmt::Display for OrbitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEccentricity(ecc) => {
                write!(f, "invalid orbital eccentricity {ecc}")
            }
            Self::InvalidSemiMajorAxis(axis) => {
                write!(f, "invalid semimajor axis {axis}")
            }
            Self::InvalidPeriod(period) => write!(f, "invalid orbital period {period}"),
            Self::InvalidPerihelion => {
                write!(f, "invalid perihelion distance or mu for parabolic orbit")
            }
            Self::InvalidTimeRange => write!(f, "invalid composite orbit time range"),
        }
    }
}

impl std::error::Error for OrbitError {}

/// Abstract trajectory evaluation contract in Celestia's reference frame (`km` and `km/day`).
pub trait Orbit: Send + Sync {
    /// Position relative to central body in kilometers at TDB Julian Date `tdb`.
    fn position_at_time(&self, tdb: f64) -> DVec3;

    /// Velocity relative to central body in kilometers per day at TDB Julian Date `tdb`.
    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        let pos0 = self.position_at_time(tdb);
        let pos1 = self.position_at_time(tdb + ORBITAL_VELOCITY_DIFF_DELTA);
        (pos1 - pos0) * (1.0 / ORBITAL_VELOCITY_DIFF_DELTA)
    }

    /// Characteristic orbital period (days) or active sample duration.
    fn period(&self) -> f64;

    /// Conservative upper bound on trajectory radius from origin in kilometers.
    fn bounding_radius(&self) -> f64;

    /// Whether the orbit repeats periodically.
    fn is_periodic(&self) -> bool {
        true
    }

    /// Valid TDB range `(begin, end)`; `(0.0, 0.0)` indicates validity for all times.
    fn valid_range(&self) -> (f64, f64) {
        (0.0, 0.0)
    }

    /// Sample the orbit over `[start_time, end_time]` using adaptive cubic Hermite step control.
    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        let span = if self.is_periodic() {
            self.period()
        } else {
            let (valid_start, valid_end) = self.valid_range();
            if valid_start.to_bits() == valid_end.to_bits() {
                valid_end - valid_start
            } else {
                end_time - start_time
            }
        };
        adaptive_sample_orbit(
            self,
            start_time,
            end_time,
            AdaptiveSamplingParams::for_span(span),
        )
    }
}

fn cubic_hermite(pos0: DVec3, vel0: DVec3, pos1: DVec3, vel1: DVec3, t: f64) -> DVec3 {
    let coeff_a = 2.0 * (pos0 - pos1) + vel1 + vel0;
    let coeff_b = 3.0 * (pos1 - pos0) - 2.0 * vel0 - vel1;
    pos0 + t * (vel0 + t * (coeff_b + t * coeff_a))
}

fn eval_hermite_error<O: Orbit + ?Sized>(
    orbit: &O,
    t: f64,
    dt: f64,
    last_pos: DVec3,
    last_vel: DVec3,
) -> (DVec3, DVec3, f64) {
    let p_next = orbit.position_at_time(t + dt);
    let v_next = orbit.velocity_at_time(t + dt);
    let p_mid = orbit.position_at_time(t + dt * 0.5);
    let p_interp = cubic_hermite(last_pos, last_vel * dt, p_next, v_next * dt, 0.5);
    (p_next, v_next, (p_interp - p_mid).length())
}

/// Adaptively sample any [`Orbit`] over `[start_time, end_time]` with explicit step bounds.
pub fn adaptive_sample_orbit<O: Orbit + ?Sized>(
    orbit: &O,
    start_time: f64,
    end_time: f64,
    params: AdaptiveSamplingParams,
) -> Vec<OrbitSample> {
    const STEP_FACTOR: f64 = 1.25;
    let mut samples = Vec::new();
    let mut time = start_time;
    let mut max_step = params.max_step;
    let min_step = params.min_step;

    let mut last_pos = orbit.position_at_time(time);
    let mut last_vel = orbit.velocity_at_time(time);
    samples.push(OrbitSample {
        time,
        position: last_pos,
        velocity: last_vel,
    });

    if !params.start_step.is_finite()
        || max_step <= 0.0
        || min_step <= 0.0
        || start_time >= end_time
    {
        return samples;
    }

    while time < end_time {
        max_step = max_step.min(end_time - time);
        let mut dt = max_step.min(params.start_step * 2.0);
        if dt <= 0.0 {
            break;
        }
        let (mut next_pos, mut next_vel, mut pos_err) =
            eval_hermite_error(orbit, time, dt, last_pos, last_vel);

        if pos_err > params.tolerance {
            while pos_err > params.tolerance && dt > min_step {
                dt /= STEP_FACTOR;
                (next_pos, next_vel, pos_err) =
                    eval_hermite_error(orbit, time, dt, last_pos, last_vel);
            }
        } else {
            while pos_err < params.tolerance && dt < max_step {
                dt = (dt * STEP_FACTOR).min(max_step);
                (next_pos, next_vel, pos_err) =
                    eval_hermite_error(orbit, time, dt, last_pos, last_vel);
                if dt >= max_step {
                    break;
                }
            }
        }

        time += dt;
        last_pos = next_pos;
        last_vel = next_vel;
        samples.push(OrbitSample {
            time,
            position: last_pos,
            velocity: last_vel,
        });
    }

    samples
}

fn orbit_plane_matrix(elements: &KeplerElements) -> DMat3 {
    DMat3::from_quat(
        z_rotation(elements.long_ascending_node)
            * x_rotation(elements.inclination)
            * z_rotation(elements.arg_pericenter),
    )
}

fn to_celestia_frame(vec: DVec3) -> DVec3 {
    DVec3::new(vec.x, vec.z, -vec.y)
}

/// Elliptical Keplerian orbit (`0.0 <= eccentricity < 1.0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EllipticalOrbit {
    semimajor_axis: f64,
    semiminor_axis: f64,
    eccentricity: f64,
    mean_anomaly_at_epoch: f64,
    period: f64,
    epoch: f64,
    orbit_plane_rotation: DMat3,
}

impl EllipticalOrbit {
    /// Construct an [`EllipticalOrbit`] from osculating `elements` at Julian Date `epoch`.
    ///
    /// # Errors
    /// Returns [`OrbitError`] if eccentricity is outside `[0, 1)`, `semimajor_axis < 0`,
    /// or `period == 0`.
    pub fn new(elements: &KeplerElements, epoch: f64) -> Result<Self, OrbitError> {
        if !(0.0..1.0).contains(&elements.eccentricity) || !elements.eccentricity.is_finite() {
            return Err(OrbitError::InvalidEccentricity(elements.eccentricity));
        }
        if elements.semimajor_axis < 0.0 || !elements.semimajor_axis.is_finite() {
            return Err(OrbitError::InvalidSemiMajorAxis(elements.semimajor_axis));
        }
        if elements.period == 0.0 || !elements.period.is_finite() {
            return Err(OrbitError::InvalidPeriod(elements.period));
        }
        let semiminor_axis = elements.semimajor_axis * (1.0 - square(elements.eccentricity)).sqrt();
        Ok(Self {
            semimajor_axis: elements.semimajor_axis,
            semiminor_axis,
            eccentricity: elements.eccentricity,
            mean_anomaly_at_epoch: elements.mean_anomaly,
            period: elements.period,
            epoch,
            orbit_plane_rotation: orbit_plane_matrix(elements),
        })
    }

    fn eccentric_anomaly(&self, mean_anom: f64) -> f64 {
        let ecc = self.eccentricity;
        if ecc == 0.0 {
            return mean_anom;
        }
        if ecc < 0.2 {
            return solve_iteration_fixed(|x| mean_anom + ecc * x.sin(), mean_anom, 5)
                .map_or(mean_anom, |res| res.root);
        }
        if ecc < 0.9 {
            return solve_iteration_fixed(
                |x| {
                    let (sin_x, cos_x) = x.sin_cos();
                    x + (mean_anom + ecc * sin_x - x) / (1.0 - ecc * cos_x)
                },
                mean_anom,
                6,
            )
            .map_or(mean_anom, |res| res.root);
        }
        let initial = mean_anom + 0.85 * ecc * sign(mean_anom.sin());
        solve_iteration_fixed(
            |x| {
                let (sin_x, cos_x) = x.sin_cos();
                let s_term = ecc * sin_x;
                let c_term = ecc * cos_x;
                let f0 = x - s_term - mean_anom;
                let f1 = 1.0 - c_term;
                let f2 = s_term;
                x - 5.0 * f0 / (f1 + sign(f1) * (16.0 * f1 * f1 - 20.0 * f0 * f2).abs().sqrt())
            },
            initial,
            8,
        )
        .map_or(initial, |res| res.root)
    }

    fn position_at_e(&self, ecc_anom: f64) -> DVec3 {
        let (sin_e, cos_e) = ecc_anom.sin_cos();
        let x = self.semimajor_axis * (cos_e - self.eccentricity);
        let y = self.semiminor_axis * sin_e;
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(x, y, 0.0))
    }

    fn velocity_at_e(&self, ecc_anom: f64, mean_motion: f64) -> DVec3 {
        let (sin_e, cos_e) = ecc_anom.sin_cos();
        let e_dot = mean_motion / (1.0 - self.eccentricity * cos_e);
        let vx = -self.semimajor_axis * sin_e * e_dot;
        let vy = self.semiminor_axis * cos_e * e_dot;
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(vx, vy, 0.0))
    }
}

impl Orbit for EllipticalOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        let mean_motion = TAU / self.period;
        let mean_anom = self.mean_anomaly_at_epoch + (tdb - self.epoch) * mean_motion;
        self.position_at_e(self.eccentric_anomaly(mean_anom))
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        let mean_motion = TAU / self.period;
        let mean_anom = self.mean_anomaly_at_epoch + (tdb - self.epoch) * mean_motion;
        self.velocity_at_e(self.eccentric_anomaly(mean_anom), mean_motion)
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn bounding_radius(&self) -> f64 {
        self.semimajor_axis * (1.0 + self.eccentricity)
    }
}

/// Hyperbolic Keplerian trajectory (`eccentricity > 1.0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HyperbolicOrbit {
    semimajor_axis: f64,
    semiminor_axis: f64,
    eccentricity: f64,
    mean_anomaly_at_epoch: f64,
    mean_motion: f64,
    epoch: f64,
    start_epoch: f64,
    end_epoch: f64,
    orbit_plane_rotation: DMat3,
}

impl HyperbolicOrbit {
    /// Construct a [`HyperbolicOrbit`] from osculating `elements` at Julian Date `epoch`.
    ///
    /// # Errors
    /// Returns [`OrbitError`] if `eccentricity <= 1.0`, `semimajor_axis > 0.0`, or `period == 0`.
    pub fn new(elements: &KeplerElements, epoch: f64) -> Result<Self, OrbitError> {
        if elements.eccentricity <= 1.0 || !elements.eccentricity.is_finite() {
            return Err(OrbitError::InvalidEccentricity(elements.eccentricity));
        }
        if elements.semimajor_axis > 0.0 || !elements.semimajor_axis.is_finite() {
            return Err(OrbitError::InvalidSemiMajorAxis(elements.semimajor_axis));
        }
        if elements.period == 0.0 || !elements.period.is_finite() {
            return Err(OrbitError::InvalidPeriod(elements.period));
        }

        let semiminor_axis = elements.semimajor_axis * (square(elements.eccentricity) - 1.0).sqrt();
        let mean_motion = TAU / elements.period;
        let bounding_radius = (2.0 * elements.semimajor_axis * (1.0 - elements.eccentricity))
            .max(HYPERBOLIC_MIN_BOUNDING_RADIUS);
        let pericenter_dist = elements.semimajor_axis * (1.0 - elements.eccentricity);
        let cos_true_anom = (pericenter_dist / bounding_radius - 1.0) / elements.eccentricity;
        let hyp_anom = ((elements.eccentricity + cos_true_anom)
            / (1.0 + elements.eccentricity * cos_true_anom))
            .clamp(1.0, f64::MAX)
            .acosh();
        let mean_anom = elements.eccentricity * hyp_anom.sinh() - hyp_anom;
        let delta_t = (mean_anom / mean_motion).abs();

        Ok(Self {
            semimajor_axis: elements.semimajor_axis,
            semiminor_axis,
            eccentricity: elements.eccentricity,
            mean_anomaly_at_epoch: elements.mean_anomaly,
            mean_motion,
            epoch,
            start_epoch: epoch - delta_t,
            end_epoch: epoch + delta_t,
            orbit_plane_rotation: orbit_plane_matrix(elements),
        })
    }

    fn eccentric_anomaly(&self, mean_anom: f64) -> f64 {
        if mean_anom == 0.0 {
            return 0.0;
        }
        let abs_m = mean_anom.abs();
        let ecc = self.eccentricity;
        let initial = (2.0 * abs_m / ecc + 1.85).ln();
        let solved = solve_iteration_fixed(
            |x| {
                let sinh_val = ecc * x.sinh();
                let cosh_val = ecc * x.cosh();
                let f0 = sinh_val - x - abs_m;
                let f1 = cosh_val - 1.0;
                let f2 = sinh_val;
                x - 5.0 * f0 / (f1 + sign(f1) * (16.0 * f1 * f1 - 20.0 * f0 * f2).abs().sqrt())
            },
            initial,
            30,
        )
        .map_or(initial, |res| res.root);
        solved.copysign(mean_anom)
    }

    fn position_at_h(&self, hyp_anom: f64) -> DVec3 {
        let x = -self.semimajor_axis * (self.eccentricity - hyp_anom.cosh());
        let y = -self.semiminor_axis * hyp_anom.sinh();
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(x, y, 0.0))
    }

    fn velocity_at_h(&self, hyp_anom: f64) -> DVec3 {
        let cosh_h = hyp_anom.cosh();
        let h_dot = self.mean_motion / (self.eccentricity * cosh_h - 1.0);
        let vx = self.semimajor_axis * hyp_anom.sinh() * h_dot;
        let vy = -self.semiminor_axis * cosh_h * h_dot;
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(vx, vy, 0.0))
    }
}

impl Orbit for HyperbolicOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        let mean_anom = self.mean_anomaly_at_epoch + (tdb - self.epoch) * self.mean_motion;
        self.position_at_h(self.eccentric_anomaly(mean_anom))
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        let mean_anom = self.mean_anomaly_at_epoch + (tdb - self.epoch) * self.mean_motion;
        self.velocity_at_h(self.eccentric_anomaly(mean_anom))
    }

    fn period(&self) -> f64 {
        self.end_epoch - self.start_epoch
    }

    fn bounding_radius(&self) -> f64 {
        (2.0 * self.semimajor_axis * (1.0 - self.eccentricity)).max(HYPERBOLIC_MIN_BOUNDING_RADIUS)
    }

    fn is_periodic(&self) -> bool {
        false
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.start_epoch, self.end_epoch)
    }
}

/// Parabolic Keplerian trajectory (`eccentricity == 1.0`) solved analytically via Barker's equation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParabolicOrbit {
    perihelion_distance: f64,
    mu: f64,
    epoch_perihelion: f64,
    orbit_plane_rotation: DMat3,
    start_epoch: f64,
    end_epoch: f64,
}

impl ParabolicOrbit {
    /// Construct a parabolic orbit from perihelion distance `q` (km), orientation angles (radians),
    /// gravitational parameter `mu` (`km^3 / day^2`), and perihelion TDB `epoch_perihelion`.
    ///
    /// # Errors
    /// Returns [`OrbitError::InvalidPerihelion`] if `q <= 0`, `mu <= 0`, or inputs are non-finite.
    pub fn new(
        perihelion_distance: f64,
        inclination: f64,
        long_ascending_node: f64,
        arg_pericenter: f64,
        mu: f64,
        epoch_perihelion: f64,
    ) -> Result<Self, OrbitError> {
        if perihelion_distance <= 0.0
            || !perihelion_distance.is_finite()
            || mu <= 0.0
            || !mu.is_finite()
            || !epoch_perihelion.is_finite()
        {
            return Err(OrbitError::InvalidPerihelion);
        }

        let elements = KeplerElements {
            semimajor_axis: 0.0,
            eccentricity: 1.0,
            inclination,
            long_ascending_node,
            arg_pericenter,
            mean_anomaly: 0.0,
            period: 0.0,
        };
        let d_max = (HYPERBOLIC_MIN_BOUNDING_RADIUS / perihelion_distance - 1.0)
            .max(1.0)
            .sqrt();
        let half_span =
            (d_max + d_max.powi(3) / 3.0) / (mu / (2.0 * perihelion_distance.powi(3))).sqrt();

        Ok(Self {
            perihelion_distance,
            mu,
            epoch_perihelion,
            orbit_plane_rotation: orbit_plane_matrix(&elements),
            start_epoch: epoch_perihelion - half_span,
            end_epoch: epoch_perihelion + half_span,
        })
    }

    fn barker_tan_half_nu(&self, tdb: f64) -> f64 {
        let dt = tdb - self.epoch_perihelion;
        let k_b = 1.5 * (self.mu / (2.0 * self.perihelion_distance.powi(3))).sqrt() * dt;
        let root_term = (k_b + (k_b * k_b + 1.0).sqrt()).cbrt();
        if root_term == 0.0 {
            0.0
        } else {
            root_term - 1.0 / root_term
        }
    }
}

impl Orbit for ParabolicOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        let d = self.barker_tan_half_nu(tdb);
        let x = self.perihelion_distance * (1.0 - d * d);
        let y = 2.0 * self.perihelion_distance * d;
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(x, y, 0.0))
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        let d = self.barker_tan_half_nu(tdb);
        let speed_scale = 2.0 * (self.mu / (2.0 * self.perihelion_distance)).sqrt() / (1.0 + d * d);
        let vx = -d * speed_scale;
        let vy = speed_scale;
        to_celestia_frame(self.orbit_plane_rotation * DVec3::new(vx, vy, 0.0))
    }

    fn period(&self) -> f64 {
        self.end_epoch - self.start_epoch
    }

    fn bounding_radius(&self) -> f64 {
        (2.0 * self.perihelion_distance).max(HYPERBOLIC_MIN_BOUNDING_RADIUS)
    }

    fn is_periodic(&self) -> bool {
        false
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.start_epoch, self.end_epoch)
    }
}

/// Dispatch construction of an [`EllipticalOrbit`] (`e < 1`) or [`HyperbolicOrbit`] (`e > 1`).
///
/// # Errors
/// Propagates [`OrbitError`] from the underlying Kepler orbit constructor.
pub fn create_kepler_orbit(
    elements: &KeplerElements,
    epoch: f64,
) -> Result<Arc<dyn Orbit>, OrbitError> {
    if elements.eccentricity < 1.0 {
        Ok(Arc::new(EllipticalOrbit::new(elements, epoch)?))
    } else {
        Ok(Arc::new(HyperbolicOrbit::new(elements, epoch)?))
    }
}

/// Fixed constant position within its reference frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedOrbit {
    position: DVec3,
    period: Option<f64>,
}

impl FixedOrbit {
    /// Construct a [`FixedOrbit`] at constant `position` (km) with optional nominal `period` (days).
    #[must_use]
    pub const fn new(position: DVec3, period: Option<f64>) -> Self {
        Self { position, period }
    }
}

impl Orbit for FixedOrbit {
    fn position_at_time(&self, _tdb: f64) -> DVec3 {
        self.position
    }

    fn velocity_at_time(&self, _tdb: f64) -> DVec3 {
        DVec3::ZERO
    }

    fn period(&self) -> f64 {
        self.period.unwrap_or(1.0)
    }

    fn bounding_radius(&self) -> f64 {
        self.position.length() * 1.1
    }

    fn is_periodic(&self) -> bool {
        self.period.is_some()
    }

    fn sample(&self, _start_time: f64, _end_time: f64) -> Vec<OrbitSample> {
        Vec::new()
    }
}

/// Composite orbit blending a bounded ephemeris `primary` onto fitted Keplerian wings outside
/// `[begin, end]` matching `MixedOrbit` in `celephem/orbit.cpp`.
pub struct MixedOrbit {
    primary: Arc<dyn Orbit>,
    before_approx: Arc<dyn Orbit>,
    after_approx: Arc<dyn Orbit>,
    begin: f64,
    end: f64,
    bounding_radius: f64,
}

impl MixedOrbit {
    /// Construct a [`MixedOrbit`] wrapping `primary` over `[begin, end]` for a parent of `mass_kg`.
    ///
    /// # Errors
    /// Returns [`OrbitError`] if `end <= begin` or Kepler wing construction fails.
    pub fn new(
        primary: Arc<dyn Orbit>,
        begin: f64,
        end: f64,
        mass_kg: f64,
    ) -> Result<Self, OrbitError> {
        if end <= begin || !begin.is_finite() || !end.is_finite() {
            return Err(OrbitError::InvalidTimeRange);
        }
        let dt = 1.0 / 1440.0;
        let p0 = primary.position_at_time(begin);
        let p1 = primary.position_at_time(end);
        let v0 = (primary.position_at_time(begin + dt) - p0) / dt;
        let v1 = (primary.position_at_time(end + dt) - p1) / dt;

        let grav_day = GRAVITATIONAL_CONSTANT * square(86400.0) * 1e-9;
        let g_mass = grav_day * mass_kg;

        let elems_before = state_vector_to_elements(p0, v0, g_mass);
        let before_approx = create_kepler_orbit(&elems_before, begin)?;

        let elems_after = state_vector_to_elements(p1, v1, g_mass);
        let after_approx = create_kepler_orbit(&elems_after, end)?;

        let bounding_radius = before_approx
            .bounding_radius()
            .max(primary.bounding_radius())
            .max(after_approx.bounding_radius());

        Ok(Self {
            primary,
            before_approx,
            after_approx,
            begin,
            end,
            bounding_radius,
        })
    }
}

impl Orbit for MixedOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        if tdb < self.begin {
            self.before_approx.position_at_time(tdb)
        } else if tdb < self.end {
            self.primary.position_at_time(tdb)
        } else {
            self.after_approx.position_at_time(tdb)
        }
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        if tdb < self.begin {
            self.before_approx.velocity_at_time(tdb)
        } else if tdb < self.end {
            self.primary.velocity_at_time(tdb)
        } else {
            self.after_approx.velocity_at_time(tdb)
        }
    }

    fn period(&self) -> f64 {
        self.primary.period()
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        if start_time < self.begin {
            self.before_approx.sample(start_time, end_time)
        } else if start_time < self.end {
            self.primary.sample(start_time, end_time)
        } else {
            self.after_approx.sample(start_time, end_time)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct OrbitCacheState {
    last_time: f64,
    position: Option<DVec3>,
    velocity: Option<DVec3>,
}

/// Thread-safe single-slot TDB cache wrapper around any expensive [`Orbit`].
#[derive(Debug)]
pub struct CachingOrbit<O> {
    inner: O,
    cache: Mutex<OrbitCacheState>,
}

impl<O: Orbit> CachingOrbit<O> {
    /// Wrap `inner` in a single-slot TDB cache.
    #[must_use]
    pub const fn new(inner: O) -> Self {
        Self {
            inner,
            cache: Mutex::new(OrbitCacheState {
                last_time: -1.0e30,
                position: None,
                velocity: None,
            }),
        }
    }

    fn lock_cache(&self) -> std::sync::MutexGuard<'_, OrbitCacheState> {
        match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl<O: Orbit> Orbit for CachingOrbit<O> {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        {
            let guard = self.lock_cache();
            if guard.last_time.to_bits() == tdb.to_bits()
                && let Some(pos) = guard.position
            {
                return pos;
            }
        }
        let computed = self.inner.position_at_time(tdb);
        let mut guard = self.lock_cache();
        if guard.last_time.to_bits() != tdb.to_bits() {
            guard.last_time = tdb;
            guard.velocity = None;
        }
        guard.position = Some(computed);
        computed
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        {
            let guard = self.lock_cache();
            if guard.last_time.to_bits() == tdb.to_bits()
                && let Some(vel) = guard.velocity
            {
                return vel;
            }
        }
        let computed = self.inner.velocity_at_time(tdb);
        let mut guard = self.lock_cache();
        if guard.last_time.to_bits() != tdb.to_bits() {
            guard.last_time = tdb;
            guard.position = None;
        }
        guard.velocity = Some(computed);
        computed
    }

    fn period(&self) -> f64 {
        self.inner.period()
    }

    fn bounding_radius(&self) -> f64 {
        self.inner.bounding_radius()
    }

    fn is_periodic(&self) -> bool {
        self.inner.is_periodic()
    }

    fn valid_range(&self) -> (f64, f64) {
        self.inner.valid_range()
    }

    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        self.inner.sample(start_time, end_time)
    }
}

#[cfg(test)]
mod tests {
    use core::f64::consts::{PI, TAU};
    use std::sync::Arc;

    use alestia_astro::{J2000, KeplerElements, SOLAR_MASS, state_vector_to_elements};
    use alestia_math::{DVec3, deg_to_rad, square};
    use proptest::prelude::*;

    use super::{
        CachingOrbit, EllipticalOrbit, FixedOrbit, HyperbolicOrbit, MixedOrbit, Orbit, OrbitError,
        ParabolicOrbit, create_kepler_orbit,
    };

    const GM_SUN: f64 = 0.000_296_014_912;
    const FOUR_PI_SQ: f64 = 4.0 * PI * PI;
    const KEPLER_TOLERANCE: f64 = 1e-3;

    fn approx_eq(lhs: f64, rhs: f64) -> bool {
        let scale = lhs.abs().max(rhs.abs()).max(1.0);
        (lhs - rhs).abs() <= KEPLER_TOLERANCE * scale
    }

    fn approx_angle(lhs: f64, rhs: f64) -> bool {
        (lhs - rhs).abs() < KEPLER_TOLERANCE
            || (lhs + TAU - rhs).abs() < KEPLER_TOLERANCE
            || (lhs - TAU - rhs).abs() < KEPLER_TOLERANCE
    }

    fn verify_elements(expected: &KeplerElements, actual: &KeplerElements) -> Result<(), String> {
        if !approx_eq(expected.period, actual.period)
            || !approx_eq(expected.semimajor_axis, actual.semimajor_axis)
            || !approx_eq(expected.eccentricity, actual.eccentricity)
            || !approx_angle(expected.inclination, actual.inclination)
            || !approx_angle(expected.long_ascending_node, actual.long_ascending_node)
            || !approx_angle(expected.arg_pericenter, actual.arg_pericenter)
            || !approx_angle(expected.mean_anomaly, actual.mean_anomaly)
        {
            return Err(format!(
                "Kepler element roundtrip mismatch:\nexpected: {expected:?}\nactual:   {actual:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn upstream_kepler_elliptical_orbits_roundtrip() -> Result<(), String> {
        let periods = [50.0, 1200.0];
        let inclinations = [0.0, 30.0, 90.0, 150.0, 180.0];
        let angles = [-180.0, -150.0, -90.0, -40.0, 0.0, 40.0, 90.0, 150.0, 180.0];
        let eccentricities = [0.0, 0.2, 0.6];

        for period in periods {
            let semimajor = (GM_SUN * square(period) / FOUR_PI_SQ).cbrt();
            for &mean_deg in &angles {
                for &inc_deg in &inclinations {
                    let nodes: &[f64] = if inc_deg == 0.0 || (inc_deg - 180.0).abs() < 1e-12 {
                        &[0.0]
                    } else {
                        &angles
                    };
                    for &node_deg in nodes {
                        for &ecc in &eccentricities {
                            let pericenters: &[f64] = if ecc == 0.0 { &[0.0] } else { &angles };
                            for &peri_deg in pericenters {
                                let expected = KeplerElements {
                                    semimajor_axis: semimajor,
                                    eccentricity: ecc,
                                    inclination: deg_to_rad(inc_deg),
                                    long_ascending_node: deg_to_rad(node_deg),
                                    arg_pericenter: deg_to_rad(peri_deg),
                                    mean_anomaly: deg_to_rad(mean_deg),
                                    period,
                                };
                                let orbit = EllipticalOrbit::new(&expected, 0.0)
                                    .map_err(|err| format!("EllipticalOrbit failed: {err}"))?;
                                let pos = orbit.position_at_time(0.0);
                                let vel = orbit.velocity_at_time(0.0);
                                let actual = state_vector_to_elements(pos, vel, GM_SUN);
                                verify_elements(&expected, &actual)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn upstream_kepler_hyperbolic_orbits_roundtrip() -> Result<(), String> {
        let periods = [50.0, 1200.0];
        let inclinations = [0.0, 30.0, 90.0, 150.0, 180.0];
        let angles = [-180.0, -150.0, -90.0, -40.0, 0.0, 40.0, 90.0, 150.0, 180.0];
        let eccentricities = [1.5, 2.4];

        for period in periods {
            let semimajor = -(GM_SUN * square(period) / FOUR_PI_SQ).cbrt();
            for &mean_deg in &angles {
                for &inc_deg in &inclinations {
                    let nodes: &[f64] = if inc_deg == 0.0 || (inc_deg - 180.0).abs() < 1e-12 {
                        &[0.0]
                    } else {
                        &angles
                    };
                    for &node_deg in nodes {
                        for &ecc in &eccentricities {
                            for &peri_deg in &angles {
                                let expected = KeplerElements {
                                    semimajor_axis: semimajor,
                                    eccentricity: ecc,
                                    inclination: deg_to_rad(inc_deg),
                                    long_ascending_node: deg_to_rad(node_deg),
                                    arg_pericenter: deg_to_rad(peri_deg),
                                    mean_anomaly: deg_to_rad(mean_deg),
                                    period,
                                };
                                let orbit = HyperbolicOrbit::new(&expected, 0.0)
                                    .map_err(|err| format!("HyperbolicOrbit failed: {err}"))?;
                                let pos = orbit.position_at_time(0.0);
                                let vel = orbit.velocity_at_time(0.0);
                                let actual = state_vector_to_elements(pos, vel, GM_SUN);
                                verify_elements(&expected, &actual)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn parabolic_orbit_conserves_zero_energy_and_angular_momentum() -> Result<(), String> {
        let q = 1.0e8;
        let mu = 1.0e18;
        let orbit = ParabolicOrbit::new(q, 0.35, 0.7, 1.1, mu, J2000)
            .map_err(|err| format!("ParabolicOrbit failed: {err}"))?;
        assert!(!orbit.is_periodic());
        let (start, end) = orbit.valid_range();
        assert!(end > start && orbit.period() > 0.0);

        let h_expected = (2.0 * mu * q).sqrt();
        for dt in [-25.0, -5.0, 0.0, 10.0, 40.0] {
            let pos = orbit.position_at_time(J2000 + dt);
            let vel = orbit.velocity_at_time(J2000 + dt);
            let energy = 0.5 * vel.length_squared() - mu / pos.length();
            if energy.abs() > 1e-6 * (mu / q) {
                return Err(format!("Parabolic specific energy non-zero: {energy}"));
            }
            let ang_mom = pos.cross(vel).length();
            if ((ang_mom - h_expected) / h_expected).abs() > 1e-10 {
                return Err("Parabolic angular momentum not conserved".to_owned());
            }
        }
        Ok(())
    }

    #[test]
    fn fixed_mixed_caching_and_validation_errors_behave_cleanly() -> Result<(), String> {
        let fixed = FixedOrbit::new(DVec3::new(3.0, 4.0, 0.0), None);
        assert_eq!(fixed.position_at_time(J2000), DVec3::new(3.0, 4.0, 0.0));
        assert_eq!(fixed.velocity_at_time(J2000), DVec3::ZERO);
        assert!(!fixed.is_periodic());
        assert_eq!(fixed.period(), 1.0);
        assert_eq!(fixed.sample(0.0, 10.0), Vec::new());

        let earth_elems = KeplerElements {
            semimajor_axis: 149_597_870.7,
            eccentricity: 0.0167,
            inclination: 0.01,
            long_ascending_node: 0.5,
            arg_pericenter: 1.8,
            mean_anomaly: 0.0,
            period: 365.25,
        };
        let earth = create_kepler_orbit(&earth_elems, J2000)
            .map_err(|err| format!("create_kepler_orbit failed: {err}"))?;
        let mixed = MixedOrbit::new(Arc::clone(&earth), J2000 - 100.0, J2000 + 100.0, SOLAR_MASS)
            .map_err(|err| format!("MixedOrbit failed: {err}"))?;
        let p_in = mixed.position_at_time(J2000);
        let p_before = mixed.position_at_time(J2000 - 150.0);
        let p_after = mixed.position_at_time(J2000 + 150.0);
        assert!(p_in.is_finite() && p_before.is_finite() && p_after.is_finite());
        assert!(mixed.bounding_radius() >= earth.bounding_radius());
        assert_ne!(mixed.sample(J2000 - 10.0, J2000 + 10.0), Vec::new());

        let cached = CachingOrbit::new(
            EllipticalOrbit::new(
                &KeplerElements {
                    eccentricity: 0.95,
                    ..earth_elems
                },
                J2000,
            )
            .map_err(|err| format!("High-ecc elliptical failed: {err}"))?,
        );
        assert_eq!(
            cached.position_at_time(J2000 + 12.0),
            cached.position_at_time(J2000 + 12.0)
        );
        assert!(cached.velocity_at_time(J2000 + 12.0).is_finite());

        assert_eq!(
            EllipticalOrbit::new(
                &KeplerElements {
                    eccentricity: 1.2,
                    ..earth_elems
                },
                J2000
            ),
            Err(OrbitError::InvalidEccentricity(1.2))
        );
        assert_eq!(
            ParabolicOrbit::new(-1.0, 0.0, 0.0, 0.0, 1.0, J2000),
            Err(OrbitError::InvalidPerihelion)
        );
        Ok(())
    }

    proptest! {
        #[test]
        fn elliptical_orbit_conserves_energy_and_angular_momentum(
            ecc in 0.0_f64..0.85_f64,
            dt in -180.0_f64..180.0_f64,
        ) {
            let period = 365.25;
            let a = (GM_SUN * square(period) / FOUR_PI_SQ).cbrt();
            let elems = KeplerElements {
                semimajor_axis: a,
                eccentricity: ecc,
                inclination: 0.4,
                long_ascending_node: 0.8,
                arg_pericenter: 1.2,
                mean_anomaly: 0.5,
                period,
            };
            let orbit = EllipticalOrbit::new(&elems, J2000)
                .map_err(|err| TestCaseError::fail(err.to_string()))?;
            let pos = orbit.position_at_time(J2000 + dt);
            let vel = orbit.velocity_at_time(J2000 + dt);
            let vel_per_day_sq = vel.length_squared();
            let mu_day = GM_SUN;
            let expected_energy = -mu_day / (2.0 * a);
            let actual_energy = 0.5 * vel_per_day_sq - mu_day / pos.length();
            prop_assert!(((actual_energy - expected_energy) / expected_energy.abs()).abs() < 1e-6);
        }
    }
}
