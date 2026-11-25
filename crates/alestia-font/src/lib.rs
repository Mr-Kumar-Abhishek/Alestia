#![forbid(unsafe_code)]
//! TrueType/OpenType glyph rasterization and LTR/RTL text layout for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[alestia_util::CRATE_VERSION];
