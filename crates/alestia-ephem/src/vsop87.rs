//! `VSOP87` analytical planetary ephemerides.
//!
//! The series are Celestia's truncated `VSOP87B` (heliocentric spherical variables, planets)
//! and `VSOP87E` (barycentric rectangular variables, Sun) tables. They are compiled in from
//! `data/vsop87.bin`, which is generated from the upstream tables by `data/gen_vsop87.py`.
//!
//! Reference: Bretagnon P., Francou G., Astron. Astrophys. 202, 309 (1988).

use core::f64::consts::{FRAC_PI_2, PI};
use core::fmt;
use std::sync::{Arc, OnceLock};

use alestia_astro::{Date, J2000, KM_PER_AU, SOLAR_MASS};
use alestia_math::DVec3;
use alestia_util::{BinaryError, BinaryReader};

use crate::orbit::{
    AdaptiveSamplingParams, MixedOrbit, Orbit, OrbitError, OrbitSample, adaptive_sample_orbit,
};

const DATA: &[u8] = include_bytes!("../data/vsop87.bin");
const BODY_COUNT: usize = 9;
const DAYS_PER_YEAR: f64 = 365.25;
const DAYS_PER_MILLENNIUM: f64 = 365_250.0;
const MAX_PREALLOC_TERMS: usize = 4096;

/// A body with a built-in `VSOP87` series.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vsop87Body {
    /// Mercury (heliocentric).
    Mercury,
    /// Venus (heliocentric).
    Venus,
    /// Earth (heliocentric).
    Earth,
    /// Mars (heliocentric).
    Mars,
    /// Jupiter (heliocentric).
    Jupiter,
    /// Saturn (heliocentric).
    Saturn,
    /// Uranus (heliocentric).
    Uranus,
    /// Neptune (heliocentric).
    Neptune,
    /// Sun (barycentric).
    Sun,
}

impl Vsop87Body {
    /// All supported bodies in table order.
    pub const ALL: [Self; BODY_COUNT] = [
        Self::Mercury,
        Self::Venus,
        Self::Earth,
        Self::Mars,
        Self::Jupiter,
        Self::Saturn,
        Self::Uranus,
        Self::Neptune,
        Self::Sun,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    /// Orbital period in days (`0.0` for the non-periodic Sun orbit).
    #[must_use]
    pub const fn period(self) -> f64 {
        match self {
            Self::Mercury => 0.2408 * DAYS_PER_YEAR,
            Self::Venus => 0.6152 * DAYS_PER_YEAR,
            Self::Earth => DAYS_PER_YEAR,
            Self::Mars => 1.8809 * DAYS_PER_YEAR,
            Self::Jupiter => 11.86 * DAYS_PER_YEAR,
            Self::Saturn => 29.4577 * DAYS_PER_YEAR,
            Self::Uranus => 84.0139 * DAYS_PER_YEAR,
            Self::Neptune => 164.793 * DAYS_PER_YEAR,
            Self::Sun => 0.0,
        }
    }

    /// Conservative bound on the orbit radius in kilometers.
    #[must_use]
    pub const fn bounding_radius(self) -> f64 {
        match self {
            // Upstream Celestia uses 6.0e7 km, which is below Mercury's aphelion
            // (about 6.98e7 km); use a bound that actually contains the orbit.
            Self::Mercury => 70_000_000.0,
            Self::Venus => 100_000_000.0,
            Self::Earth => 160_000_000.0,
            Self::Mars => 240_000_000.0,
            Self::Jupiter => 800_000_000.0,
            Self::Saturn => 1.5e9,
            Self::Uranus => 3.0e9,
            Self::Neptune => 4.7e9,
            Self::Sun => 2_000_000.0,
        }
    }

    /// Calendar years `(begin, end)` over which the series is used directly; outside this
    /// window the orbit is extrapolated with a Kepler orbit.
    #[must_use]
    pub const fn valid_years(self) -> (i32, i32) {
        match self {
            Self::Sun => (-4000, 6000),
            _ => (-4000, 4000),
        }
    }
}

/// Errors from building a `VSOP87` orbit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Vsop87Error {
    /// The embedded series data failed to decode.
    CorruptData,
    /// The wrapping [`MixedOrbit`] could not be constructed.
    Orbit(OrbitError),
}

impl fmt::Display for Vsop87Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptData => write!(f, "embedded VSOP87 series data is corrupt"),
            Self::Orbit(err) => write!(f, "VSOP87 orbit construction failed: {err}"),
        }
    }
}

impl std::error::Error for Vsop87Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Orbit(err) => Some(err),
            Self::CorruptData => None,
        }
    }
}

impl From<OrbitError> for Vsop87Error {
    fn from(err: OrbitError) -> Self {
        Self::Orbit(err)
    }
}

/// One power-of-`t` series: terms of `(A, B, C)` summed as `A * cos(B + C * t)`.
type Series = Vec<[f64; 3]>;

#[derive(Debug)]
struct BodyTables {
    vars: [Vec<Series>; 3],
}

fn read_count(reader: &mut BinaryReader<'_>) -> Result<usize, BinaryError> {
    Ok(usize::try_from(reader.read_le_u32()?).unwrap_or(0))
}

fn parse_tables(bytes: &[u8]) -> Result<Vec<BodyTables>, BinaryError> {
    let mut reader = BinaryReader::new(bytes);
    let mut bodies = Vec::with_capacity(BODY_COUNT);
    for _ in 0..BODY_COUNT {
        let mut vars: [Vec<Series>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for var in &mut vars {
            let series_count = read_count(&mut reader)?;
            for _ in 0..series_count {
                let term_count = read_count(&mut reader)?;
                let mut terms = Vec::with_capacity(term_count.min(MAX_PREALLOC_TERMS));
                for _ in 0..term_count {
                    let amplitude = reader.read_le_f64()?;
                    let phase = reader.read_le_f64()?;
                    let frequency = reader.read_le_f64()?;
                    terms.push([amplitude, phase, frequency]);
                }
                var.push(terms);
            }
        }
        bodies.push(BodyTables { vars });
    }
    Ok(bodies)
}

fn tables() -> Result<&'static [BodyTables], Vsop87Error> {
    static TABLES: OnceLock<Option<Vec<BodyTables>>> = OnceLock::new();
    TABLES
        .get_or_init(|| parse_tables(DATA).ok())
        .as_deref()
        .ok_or(Vsop87Error::CorruptData)
}

fn sum_series(series: &[[f64; 3]], t: f64) -> f64 {
    series
        .iter()
        .map(|&[amplitude, phase, frequency]| amplitude * frequency.mul_add(t, phase).cos())
        .sum()
}

/// Evaluate `sum_k t^k * series[k](t)`.
fn eval_variable(series: &[Series], t: f64) -> f64 {
    let mut power = 1.0;
    let mut total = 0.0;
    for entry in series {
        total += sum_series(entry, t) * power;
        power *= t;
    }
    total
}

/// `VSOP87` orbit evaluating spherical (planets) or rectangular (Sun) series.
#[derive(Debug)]
pub struct Vsop87Orbit {
    tables: &'static BodyTables,
    rectangular: bool,
    period: f64,
    bounding_radius: f64,
}

impl Vsop87Orbit {
    /// Build the raw series orbit for `body`.
    ///
    /// # Errors
    /// Returns [`Vsop87Error::CorruptData`] if the embedded tables fail to decode.
    pub fn new(body: Vsop87Body) -> Result<Self, Vsop87Error> {
        let tables = tables()?
            .get(body.index())
            .ok_or(Vsop87Error::CorruptData)?;
        Ok(Self {
            tables,
            rectangular: body == Vsop87Body::Sun,
            period: body.period(),
            bounding_radius: body.bounding_radius(),
        })
    }
}

impl Orbit for Vsop87Orbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        // Julian millennia since J2000.0.
        let t = (tdb - J2000) / DAYS_PER_MILLENNIUM;
        let [first, second, third] = &self.tables.vars;
        if self.rectangular {
            let x = eval_variable(first, t) * KM_PER_AU;
            let y = eval_variable(second, t) * KM_PER_AU;
            let z = eval_variable(third, t) * KM_PER_AU;
            // Convert VSOP87 ecliptic axes to Celestia's internal frame.
            DVec3::new(x, z, -y)
        } else {
            let longitude = eval_variable(first, t) + PI;
            let latitude = eval_variable(second, t) - FRAC_PI_2;
            let radius = eval_variable(third, t) * KM_PER_AU;
            DVec3::new(
                longitude.cos() * latitude.sin() * radius,
                latitude.cos() * radius,
                -longitude.sin() * latitude.sin() * radius,
            )
        }
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn is_periodic(&self) -> bool {
        self.period > 0.0
    }

    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        let params = if self.rectangular {
            AdaptiveSamplingParams::for_span(end_time - start_time)
        } else {
            // Equal start/min/max steps make adaptive sampling behave like uniform sampling,
            // which is much faster than the default for these smooth series.
            let step = self.period / 150.0;
            AdaptiveSamplingParams {
                tolerance: 1.0,
                start_step: step,
                min_step: step,
                max_step: step,
            }
        };
        adaptive_sample_orbit(self, start_time, end_time, params)
    }
}

/// Build the `VSOP87` orbit for `body`, valid over [`Vsop87Body::valid_years`] and
/// extrapolated with a Kepler orbit outside that window.
///
/// # Errors
/// Returns [`Vsop87Error`] if the embedded data is corrupt or the wrapper cannot be built.
pub fn create_vsop87_orbit(body: Vsop87Body) -> Result<MixedOrbit, Vsop87Error> {
    let orbit = Vsop87Orbit::new(body)?;
    let (begin_year, end_year) = body.valid_years();
    let begin = Date::new(begin_year, 1, 1).to_julian_date();
    let end = Date::new(end_year, 1, 1).to_julian_date();
    Ok(MixedOrbit::new(Arc::new(orbit), begin, end, SOLAR_MASS)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_decodes_and_truncation_is_rejected() -> Result<(), Vsop87Error> {
        let parsed = tables()?;
        assert_eq!(parsed.len(), BODY_COUNT);
        assert!(parse_tables(&[]).is_err());
        assert!(parse_tables(&DATA[..DATA.len() - 1]).is_err());
        Ok(())
    }

    #[test]
    fn earth_matches_known_j2000_distance() -> Result<(), Vsop87Error> {
        let orbit = Vsop87Orbit::new(Vsop87Body::Earth)?;
        let pos = orbit.position_at_time(J2000);
        let au = pos.length() / KM_PER_AU;
        assert!((0.982..0.985).contains(&au), "distance {au} AU");
        // The ecliptic normal is the Y axis in Celestia's frame.
        assert!(pos.y.abs() < 1.0e-3 * KM_PER_AU);
        Ok(())
    }

    #[test]
    fn sun_is_near_barycenter() -> Result<(), Vsop87Error> {
        let orbit = Vsop87Orbit::new(Vsop87Body::Sun)?;
        assert!(!orbit.is_periodic());
        let pos = orbit.position_at_time(J2000);
        assert!(pos.is_finite());
        assert!(pos.length() < 0.02 * KM_PER_AU);
        Ok(())
    }

    #[test]
    fn every_body_stays_inside_bounding_radius() -> Result<(), Vsop87Error> {
        for body in Vsop87Body::ALL {
            let orbit = create_vsop87_orbit(body)?;
            let (begin, end) = body.valid_years();
            assert!(begin < end);
            for offset in [0.0, 1_000.0, 20_000.0] {
                let pos = orbit.position_at_time(J2000 + offset);
                assert!(pos.is_finite(), "{body:?} not finite");
                assert!(
                    pos.length() < body.bounding_radius(),
                    "{body:?} outside bounding radius"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn spherical_orbit_samples_uniformly() -> Result<(), Vsop87Error> {
        let orbit = Vsop87Orbit::new(Vsop87Body::Mercury)?;
        let samples = orbit.sample(J2000, J2000 + orbit.period());
        assert_ne!(samples.len(), 0);
        Ok(())
    }
}
