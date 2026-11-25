#![deny(unsafe_code)]
//! Orbital mechanics, analytical ephemerides (`VSOP87`, `TASS17`), sampled trajectories,
//! JPL binary ephemerides, and Earth/planetary rotations for Alestia.
//!
//! # Examples
//!
//! ```
//! use alestia_astro::{J2000, KeplerElements};
//! use alestia_ephem::{EllipticalOrbit, Orbit};
//!
//! let elements = KeplerElements {
//!     semimajor_axis: 149_597_870.7,
//!     eccentricity: 0.0167,
//!     inclination: 0.0,
//!     long_ascending_node: 0.0,
//!     arg_pericenter: 0.0,
//!     mean_anomaly: 0.0,
//!     period: 365.25,
//! };
//! let orbit = EllipticalOrbit::new(&elements, J2000)?;
//! assert!(orbit.position_at_time(J2000).length() > 1.4e8);
//! # Ok::<(), alestia_ephem::OrbitError>(())
//! ```

pub mod jpleph;
pub mod nutation;
pub mod orbit;
pub mod precession;
pub mod rotation;
pub mod sampled;
#[cfg(feature = "spice")]
pub mod spice;
pub mod vsop87;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[
    alestia_astro::CRATE_VERSION,
    alestia_math::CRATE_VERSION,
    alestia_util::CRATE_VERSION,
];

pub use nutation::{NutationAngles, nutation_iau2000b};
pub use orbit::{
    AdaptiveSamplingParams, CachingOrbit, EllipticalOrbit, FixedOrbit, HyperbolicOrbit, MixedOrbit,
    Orbit, OrbitError, OrbitSample, ParabolicOrbit, adaptive_sample_orbit, create_kepler_orbit,
};
pub use precession::{
    EclipticAngles, EclipticPole, EquatorialPrecessionAngles, PrecessionAngles,
    ecliptic_precession_angles_p03, ecliptic_precession_p03, ecliptic_precession_p03_lp,
    equatorial_precession_angles_p03, prec_obliquity_p03, prec_obliquity_p03_lp,
};
pub use rotation::{
    CachingRotationModel, ConstantOrientation, PrecessingRotationModel, RotationModel,
    UniformRotationModel,
};
