#![deny(unsafe_code)]
//! Orbital mechanics, analytical ephemerides (VSOP87, TASS17), sampled trajectories, and rotations for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[
    alestia_astro::CRATE_VERSION,
    alestia_math::CRATE_VERSION,
    alestia_util::CRATE_VERSION,
];
