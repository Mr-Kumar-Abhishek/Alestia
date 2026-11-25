//! Planetary and spacecraft rotation models (`ConstantOrientation`, `UniformRotationModel`,
//! `PrecessingRotationModel`, `CachingRotationModel`) matching `celephem/rotation.h` and `rotation.cpp` (`FR-014`).

use core::f64::consts::TAU;
use std::sync::Mutex;

use alestia_math::{DQuat, DVec3, x_rotation, y_rotation};

const ANGULAR_VEL_DIFF_DELTA: f64 = 1.0 / 1440.0;

/// Orientation model describing an object's attitude as a function of TDB Julian Date.
pub trait RotationModel: Send + Sync {
    /// Rotation about the primary spin axis at `tdb`.
    fn spin(&self, tdb: f64) -> DQuat;

    /// Orientation of the equatorial plane (normal to primary axis) at `tdb`.
    fn equator_orientation_at_time(&self, _tdb: f64) -> DQuat {
        DQuat::IDENTITY
    }

    /// Combined orientation `spin(tdb) * equator_orientation_at_time(tdb)`.
    fn orientation_at_time(&self, tdb: f64) -> DQuat {
        self.spin(tdb) * self.equator_orientation_at_time(tdb)
    }

    /// Instantaneous angular velocity vector in `radians / day` at `tdb`.
    fn angular_velocity_at_time(&self, tdb: f64) -> DVec3 {
        let q0 = self.orientation_at_time(tdb);
        let q1 = self.orientation_at_time(tdb + ANGULAR_VEL_DIFF_DELTA);
        let delta_q = q1 * q0.conjugate();
        let (axis, angle) = delta_q.to_axis_angle();
        axis * (angle / ANGULAR_VEL_DIFF_DELTA)
    }

    /// Rotation period in days (`0.0` if constant or aperiodic).
    fn period(&self) -> f64;

    /// Whether the rotation repeats periodically.
    fn is_periodic(&self) -> bool;

    /// Valid TDB time span `(begin, end)`; `(0.0, 0.0)` indicates validity for all times.
    fn valid_range(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
}

/// Fixed constant orientation within the reference frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstantOrientation {
    orientation: DQuat,
}

impl Default for ConstantOrientation {
    fn default() -> Self {
        Self::identity()
    }
}

impl ConstantOrientation {
    /// Construct a constant orientation model from quaternion `orientation`.
    #[must_use]
    pub const fn new(orientation: DQuat) -> Self {
        Self { orientation }
    }

    /// Constant identity orientation.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            orientation: DQuat::IDENTITY,
        }
    }
}

impl RotationModel for ConstantOrientation {
    fn spin(&self, _tdb: f64) -> DQuat {
        self.orientation
    }

    fn angular_velocity_at_time(&self, _tdb: f64) -> DVec3 {
        DVec3::ZERO
    }

    fn period(&self) -> f64 {
        0.0
    }

    fn is_periodic(&self) -> bool {
        false
    }
}

/// Constant-rate sidereal spin about a fixed rotational axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformRotationModel {
    period: f64,
    offset: f32,
    epoch: f64,
    inclination: f32,
    ascending_node: f32,
}

impl UniformRotationModel {
    /// Construct a uniform rotation model matching `UniformRotationModel` in `celephem/rotation.cpp`.
    #[must_use]
    pub const fn new(
        period: f64,
        offset: f32,
        epoch: f64,
        inclination: f32,
        ascending_node: f32,
    ) -> Self {
        Self {
            period,
            offset,
            epoch,
            inclination,
            ascending_node,
        }
    }
}

impl RotationModel for UniformRotationModel {
    fn spin(&self, tdb: f64) -> DQuat {
        if self.period == 0.0 || !self.period.is_finite() {
            return DQuat::IDENTITY;
        }
        let rotations = (tdb - self.epoch) / self.period;
        let remainder = (rotations - rotations.floor()) + 0.5;
        y_rotation(-remainder * TAU - f64::from(self.offset))
    }

    fn equator_orientation_at_time(&self, _tdb: f64) -> DQuat {
        x_rotation(-f64::from(self.inclination)) * y_rotation(-f64::from(self.ascending_node))
    }

    fn angular_velocity_at_time(&self, tdb: f64) -> DVec3 {
        if self.period == 0.0 || !self.period.is_finite() {
            return DVec3::ZERO;
        }
        let axis = self.equator_orientation_at_time(tdb).conjugate() * DVec3::Y;
        axis * (TAU / self.period)
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn is_periodic(&self) -> bool {
        true
    }
}

/// Uniform spin combined with steady precession of the rotational axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrecessingRotationModel {
    period: f64,
    offset: f32,
    epoch: f64,
    inclination: f32,
    ascending_node: f32,
    precession_period: f64,
}

impl PrecessingRotationModel {
    /// Construct a precessing rotation model matching `PrecessingRotationModel` in `celephem/rotation.cpp`.
    #[must_use]
    pub const fn new(
        period: f64,
        offset: f32,
        epoch: f64,
        inclination: f32,
        ascending_node: f32,
        precession_period: f64,
    ) -> Self {
        Self {
            period,
            offset,
            epoch,
            inclination,
            ascending_node,
            precession_period,
        }
    }
}

impl RotationModel for PrecessingRotationModel {
    fn spin(&self, tdb: f64) -> DQuat {
        if self.period == 0.0 || !self.period.is_finite() {
            return DQuat::IDENTITY;
        }
        let rotations = (tdb - self.epoch) / self.period;
        let remainder = (rotations - rotations.floor()) + 0.5;
        y_rotation(-remainder * TAU - f64::from(self.offset))
    }

    fn equator_orientation_at_time(&self, tdb: f64) -> DQuat {
        let node_of_date = if self.precession_period == 0.0 {
            f64::from(self.ascending_node)
        } else {
            f64::from(self.ascending_node) - (TAU / self.precession_period) * (tdb - self.epoch)
        };
        x_rotation(-f64::from(self.inclination)) * y_rotation(-node_of_date)
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn is_periodic(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug)]
struct RotationCacheState {
    last_time: f64,
    spin: Option<DQuat>,
    equator: Option<DQuat>,
    angular_velocity: Option<DVec3>,
}

/// Thread-safe single-slot time cache wrapper around any [`RotationModel`].
#[derive(Debug)]
pub struct CachingRotationModel<R> {
    inner: R,
    cache: Mutex<RotationCacheState>,
}

impl<R: RotationModel> CachingRotationModel<R> {
    /// Wrap `inner` in a single-slot TDB cache.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self {
            inner,
            cache: Mutex::new(RotationCacheState {
                last_time: -1.0e30,
                spin: None,
                equator: None,
                angular_velocity: None,
            }),
        }
    }

    fn lock_cache(&self) -> std::sync::MutexGuard<'_, RotationCacheState> {
        match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl<R: RotationModel> RotationModel for CachingRotationModel<R> {
    fn spin(&self, tdb: f64) -> DQuat {
        {
            let guard = self.lock_cache();
            if guard.last_time == tdb
                && let Some(cached) = guard.spin
            {
                return cached;
            }
        }
        let computed = self.inner.spin(tdb);
        let mut guard = self.lock_cache();
        if guard.last_time != tdb {
            guard.last_time = tdb;
            guard.equator = None;
            guard.angular_velocity = None;
        }
        guard.spin = Some(computed);
        computed
    }

    fn equator_orientation_at_time(&self, tdb: f64) -> DQuat {
        {
            let guard = self.lock_cache();
            if guard.last_time == tdb
                && let Some(cached) = guard.equator
            {
                return cached;
            }
        }
        let computed = self.inner.equator_orientation_at_time(tdb);
        let mut guard = self.lock_cache();
        if guard.last_time != tdb {
            guard.last_time = tdb;
            guard.spin = None;
            guard.angular_velocity = None;
        }
        guard.equator = Some(computed);
        computed
    }

    fn angular_velocity_at_time(&self, tdb: f64) -> DVec3 {
        {
            let guard = self.lock_cache();
            if guard.last_time == tdb
                && let Some(cached) = guard.angular_velocity
            {
                return cached;
            }
        }
        let computed = self.inner.angular_velocity_at_time(tdb);
        let mut guard = self.lock_cache();
        if guard.last_time != tdb {
            guard.last_time = tdb;
            guard.spin = None;
            guard.equator = None;
        }
        guard.angular_velocity = Some(computed);
        computed
    }

    fn period(&self) -> f64 {
        self.inner.period()
    }

    fn is_periodic(&self) -> bool {
        self.inner.is_periodic()
    }

    fn valid_range(&self) -> (f64, f64) {
        self.inner.valid_range()
    }
}

#[cfg(test)]
mod tests {
    use core::f64::consts::TAU;

    use alestia_astro::J2000;
    use alestia_math::{DQuat, DVec3};

    use super::{
        CachingRotationModel, ConstantOrientation, PrecessingRotationModel, RotationModel,
        UniformRotationModel,
    };

    #[test]
    fn constant_and_uniform_rotations_match_analytical_spin() -> Result<(), String> {
        let fixed = ConstantOrientation::default();
        assert_eq!(fixed.spin(J2000), DQuat::IDENTITY);
        assert_eq!(fixed.angular_velocity_at_time(J2000), DVec3::ZERO);
        assert!(!fixed.is_periodic());
        assert_eq!(fixed.period(), 0.0);

        let uniform = UniformRotationModel::new(1.0, 0.0, J2000, 0.0, 0.0);
        assert!(uniform.is_periodic());
        assert_eq!(uniform.period(), 1.0);
        let q0 = uniform.orientation_at_time(J2000);
        let q1 = uniform.orientation_at_time(J2000 + 1.0);
        if (q0 - q1).length() > 1e-12 && (q0 + q1).length() > 1e-12 {
            return Err("Uniform rotation should repeat after one full period".to_owned());
        }
        let omega = uniform.angular_velocity_at_time(J2000);
        if (omega.length() - TAU).abs() > 1e-11 {
            return Err(format!("Unexpected angular velocity magnitude: {omega:?}"));
        }
        Ok(())
    }

    #[test]
    fn precessing_and_cached_rotations_behave_consistently() -> Result<(), String> {
        let precessing = PrecessingRotationModel::new(1.0, 0.1, J2000, 0.4, 0.2, 1000.0);
        let cached = CachingRotationModel::new(precessing);
        assert!(cached.is_periodic());
        assert_eq!(cached.period(), 1.0);
        assert_eq!(cached.valid_range(), (0.0, 0.0));

        let q_a = cached.orientation_at_time(J2000 + 25.0);
        let q_b = cached.orientation_at_time(J2000 + 25.0);
        if (q_a - q_b).length() > 1e-15 {
            return Err("Cached orientation differed on repeated query".to_owned());
        }
        let vel = cached.angular_velocity_at_time(J2000 + 25.0);
        if !vel.is_finite() || vel.length() == 0.0 {
            return Err("Precessing angular velocity should be non-zero finite".to_owned());
        }
        let eq0 = cached.equator_orientation_at_time(J2000);
        let eq1 = cached.equator_orientation_at_time(J2000 + 250.0);
        if (eq0 - eq1).length() < 1e-3 {
            return Err(
                "Precessing equator failed to advance over 0.25 precession period".to_owned(),
            );
        }
        Ok(())
    }
}
