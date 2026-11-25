#![deny(unsafe_code)]
//! `wgpu`/WGSL linear-HDR astronomical renderer for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[
    alestia_engine::CRATE_VERSION,
    alestia_font::CRATE_VERSION,
    alestia_image::CRATE_VERSION,
    alestia_model::CRATE_VERSION,
];
