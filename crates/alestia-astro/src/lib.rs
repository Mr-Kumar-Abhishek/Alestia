#![forbid(unsafe_code)]
//! Astronomical timekeeping (JD/UTC/TDB) and unit/photometric conversions for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] =
    &[alestia_math::CRATE_VERSION, alestia_util::CRATE_VERSION];
