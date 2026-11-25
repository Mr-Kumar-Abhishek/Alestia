#![forbid(unsafe_code)]
//! Application orchestration, `celestia.cfg` loader, catalog loaders, and `EclipseFinder` for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] =
    &[alestia_engine::CRATE_VERSION, alestia_script::CRATE_VERSION];
