#![deny(unsafe_code)]
//! Legacy `.cel` command and sandboxed `.celx` Lua scripting runtime for Alestia.

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[alestia_engine::CRATE_VERSION];
