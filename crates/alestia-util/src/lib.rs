#![forbid(unsafe_code)]
//! Core utilities for Alestia (`R128`, catalog tokenizer/parser, natural sort, Greek, binary I/O, color, logger).

pub mod associative_array;
pub mod color;
pub mod parser;
pub mod r128;
pub mod tokenizer;

pub use associative_array::{AssociativeArray, Value, ValueType};
pub use color::{Color, linear_to_srgb, srgb_to_linear};
pub use parser::{ParseError, Parser};
pub use r128::R128;
pub use tokenizer::{TokenType, Tokenizer};
