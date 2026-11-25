#![deny(unsafe_code)]
//! WebAssembly (`wasm32-unknown-unknown`) bindings and browser entrypoint for Alestia.

use std::fmt::Write as _;

use alestia_util::{
    AssociativeArray, Color, Parser, R128, Value, replace_greek_letter_abbr, strnatcasecmp,
};
use wasm_bindgen::prelude::wasm_bindgen;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return a human-readable version banner identifying the loaded Alestia WASM build.
#[must_use]
#[wasm_bindgen]
pub fn alestia_version() -> String {
    format!(
        "Alestia v{CRATE_VERSION} (core {}, util {})",
        alestia_core::CRATE_VERSION,
        alestia_util::CRATE_VERSION,
    )
}

/// Encode a floating-point coordinate using Alestia's 64.64 fixed-point [`R128`] Base64 codec.
#[must_use]
#[wasm_bindgen]
pub fn r128_encode_base64(value: f64) -> String {
    R128::from_f64(value).encode_base64()
}

/// Decode an [`R128`] Base64 coordinate string back into `f64`.
///
/// # Errors
/// Returns a descriptive error message when `encoded` is not a valid `R128` Base64 string.
#[wasm_bindgen]
pub fn r128_decode_base64(encoded: &str) -> Result<f64, String> {
    R128::decode_base64(encoded)
        .map(R128::to_f64)
        .ok_or_else(|| format!("Invalid R128 Base64 coordinate: {encoded:?}"))
}

/// Expand Celestia 3-letter Greek Bayer abbreviations (e.g., `"ALF Cen"` -> `"α Cen"`), one designation per line.
#[must_use]
#[wasm_bindgen]
pub fn expand_greek_bayer(input: &str) -> String {
    input
        .lines()
        .map(|line| replace_greek_letter_abbr(line.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a `#rrggbb` or `#rrggbbaa` hexadecimal color string into normalized sRGB and RGBA summary text.
///
/// # Errors
/// Returns an error if `spec` is not a valid 6- or 8-digit hex color prefixed with `#`.
#[wasm_bindgen]
pub fn parse_color_hex(spec: &str) -> Result<String, String> {
    let color = Color::parse_hex(spec.trim())
        .ok_or_else(|| format!("Invalid hex color (expected #rrggbb or #rrggbbaa): {spec:?}"))?;
    let [red, green, blue, alpha] = color.to_rgba8();
    let linear = color.to_linear_rgba();
    Ok(format!(
        "hex=#{red:02x}{green:02x}{blue:02x}{alpha:02x} rgba8=({red}, {green}, {blue}, {alpha}) srgb=({:.4}, {:.4}, {:.4}, {:.4}) linear=({:.4}, {:.4}, {:.4}, {:.4})",
        color.red(),
        color.green(),
        color.blue(),
        color.alpha(),
        linear[0],
        linear[1],
        linear[2],
        linear[3],
    ))
}

/// Parse a Celestia catalog block (`{ Key Value ... }`) into a formatted property summary.
///
/// # Errors
/// Returns a formatted [`alestia_util::ParseError`] message if `source` is not a valid catalog hash.
#[wasm_bindgen]
pub fn parse_catalog_object(source: &str) -> Result<String, String> {
    let mut parser = Parser::from_str(source);
    let table = parser.read_hash().map_err(|err| err.to_string())?;
    let mut output = String::new();
    format_hash(&table, 0, &mut output);
    Ok(output)
}

/// Sort newline-delimited astronomical catalog names using Celestia's natural case-insensitive sort.
#[must_use]
#[wasm_bindgen]
pub fn natural_sort_lines(input: &str) -> String {
    let mut lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    lines.sort_by(|lhs, rhs| strnatcasecmp(lhs, rhs));
    lines.join("\n")
}

fn format_hash(table: &AssociativeArray, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    for (key, value) in table.iter() {
        let _ = writeln!(out, "{pad}{key}: {}", format_value(value, indent));
    }
}

fn format_value(value: &Value, indent: usize) -> String {
    match value {
        Value::Number(num) => format!("{num}"),
        Value::String(text) => format!("{text:?}"),
        Value::Boolean(flag) => format!("{flag}"),
        Value::Array(items) => {
            let formatted: Vec<String> = items
                .iter()
                .map(|item| format_value(item, indent))
                .collect();
            format!("[{}]", formatted.join(", "))
        }
        Value::Hash(nested) => {
            let mut buf = String::from("{\n");
            format_hash(nested, indent + 2, &mut buf);
            let pad = " ".repeat(indent);
            let _ = write!(buf, "{pad}}}");
            buf
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CRATE_VERSION, alestia_version, expand_greek_bayer, natural_sort_lines,
        parse_catalog_object, parse_color_hex, r128_decode_base64, r128_encode_base64,
    };

    #[test]
    fn version_banner_mentions_crate_versions() -> Result<(), String> {
        let banner = alestia_version();
        if !banner.contains(CRATE_VERSION) || !banner.contains("Alestia") {
            return Err(format!("Unexpected version banner: {banner}"));
        }
        Ok(())
    }

    #[test]
    fn r128_roundtrips_and_rejects_invalid_base64() -> Result<(), String> {
        let encoded = r128_encode_base64(42.25);
        let decoded = r128_decode_base64(&encoded)?;
        if (decoded - 42.25).abs() > 1e-12 {
            return Err(format!("R128 round-trip drifted: {decoded}"));
        }
        if r128_decode_base64("!!!invalid!!!").is_ok() {
            return Err("Expected invalid Base64 input to fail".to_owned());
        }
        Ok(())
    }

    #[test]
    fn expands_greek_bayer_abbreviations() -> Result<(), String> {
        let single = expand_greek_bayer("XI12 Bar");
        if single != "ξ¹² Bar" {
            return Err(format!("Unexpected single Bayer expansion: {single}"));
        }
        let multi = expand_greek_bayer("ALF Cen\nXI12 Bar");
        if multi != "α Cen\nξ¹² Bar" {
            return Err(format!("Unexpected multiline Bayer expansion: {multi}"));
        }
        Ok(())
    }

    #[test]
    fn parses_valid_and_invalid_hex_colors() -> Result<(), String> {
        let parsed = parse_color_hex(" #1e88e5 ")?;
        if !parsed.contains("hex=#1e88e5ff") || !parsed.contains("rgba8=(30, 136, 229, 255)") {
            return Err(format!("Unexpected color summary: {parsed}"));
        }
        if parse_color_hex("not-a-color").is_ok() {
            return Err("Expected invalid color spec to fail".to_owned());
        }
        Ok(())
    }

    #[test]
    fn parses_catalog_object_with_nested_values() -> Result<(), String> {
        let sample = r#"{
            Class "planet"
            Radius 6378.137
            Oblateness true
            Color [0.4 0.6 1.0]
            Atmosphere { Height 60 }
        }"#;
        let parsed = parse_catalog_object(sample)?;
        for expected in [
            "Atmosphere: {",
            "Height: 60",
            "Class: \"planet\"",
            "Color: [0.4, 0.6, 1]",
            "Oblateness: true",
            "Radius: 6378.137",
        ] {
            if !parsed.contains(expected) {
                return Err(format!("Missing {expected:?} in parsed output:\n{parsed}"));
            }
        }
        if parse_catalog_object("{ Unterminated").is_ok() {
            return Err("Expected unterminated catalog hash to fail".to_owned());
        }
        Ok(())
    }

    #[test]
    fn natural_sorts_astronomical_lines() -> Result<(), String> {
        let sorted = natural_sort_lines("M31\n\n m2 \nM10\nM1\n");
        if sorted != "M1\nm2\nM10\nM31" {
            return Err(format!("Unexpected natural sort order: {sorted:?}"));
        }
        Ok(())
    }
}
