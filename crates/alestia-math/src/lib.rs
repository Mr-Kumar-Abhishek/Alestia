#![forbid(unsafe_code)]
//! 3D computational geometry, quaternions, frustums, intersections, and root solvers for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[alestia_util::CRATE_VERSION];
