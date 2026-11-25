#![forbid(unsafe_code)]
//! 3D mesh and material loaders (CMOD ASCII/binary, bounds-checked 3DS) for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] =
    &[alestia_math::CRATE_VERSION, alestia_util::CRATE_VERSION];
