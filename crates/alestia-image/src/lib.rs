#![forbid(unsafe_code)]
//! Texture and image decoders (PNG, JPEG, DDS DXT1/3/5, optional AVIF) for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[alestia_util::CRATE_VERSION];
