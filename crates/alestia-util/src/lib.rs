#![forbid(unsafe_code)]
//! Core utilities for Alestia (`R128`, catalog tokenizer/parser, natural sort, Greek, binary I/O, color, logger).
//!
//! # Examples
//!
//! ```
//! use alestia_util::{Parser, R128, replace_greek_letter_abbr};
//!
//! let coord = R128::from_f64(1.5);
//! let encoded = coord.encode_base64();
//! assert_eq!(R128::decode_base64(&encoded), Some(coord));
//!
//! assert_eq!(replace_greek_letter_abbr("XI12 Bar"), "ξ¹² Bar");
//!
//! let mut parser = Parser::from_str(r#"{ Radius 6378.137 Class "planet" }"#);
//! let obj = parser.read_hash()?;
//! assert_eq!(obj.get_string("Class"), Some("planet"));
//! # Ok::<(), alestia_util::ParseError>(())
//! ```

pub mod associative_array;
pub mod binary;
pub mod color;
pub mod greek;
pub mod logger;
pub mod parser;
pub mod r128;
pub mod strnatcmp;
pub mod tokenizer;
pub mod unicode;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use associative_array::{AssociativeArray, Value, ValueType};
pub use binary::{BinaryError, BinaryReader, BinaryWriter};
pub use color::{Color, linear_to_srgb, srgb_to_linear};
pub use greek::{canonical_greek_abbreviation, replace_greek_letter, replace_greek_letter_abbr};
pub use logger::{Level, LogRingBuffer, Logger};
pub use parser::{ParseError, Parser};
pub use r128::R128;
pub use strnatcmp::{strnatcasecmp, strnatcmp};
pub use tokenizer::{TokenType, Tokenizer};
pub use unicode::{LayoutDirection, compare_ignoring_case, sanitize_utf8_bytes, utf8_starts_with};
