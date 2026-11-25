#![forbid(unsafe_code)]
//! Headless astronomical simulation engine (`Universe`, `Observer`, Octrees, `DepthPartition`) for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[
    alestia_astro::CRATE_VERSION,
    alestia_ephem::CRATE_VERSION,
    alestia_math::CRATE_VERSION,
    alestia_util::CRATE_VERSION,
];
