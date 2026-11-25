//! Recursive-descent parser turning [`Tokenizer`] streams into [`Value`] and [`AssociativeArray`] trees.
//!
//! Matches `celestia::util::Parser` (`src/celutil/parser.cpp`).

use std::error::Error;
use std::fmt;

use crate::{AssociativeArray, TokenType, Tokenizer, Value};

/// Errors produced when parsing Celestia catalog or config syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number where the syntax error was encountered.
    pub line: usize,
    /// Human-readable error description.
    pub message: String,
}

impl ParseError {
    /// Construct a new [`ParseError`].
    #[must_use]
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Error for ParseError {}

/// Recursive-descent parser for Celestia key-value blocks, arrays, strings, numbers, and booleans.
pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> Parser<'a> {
    /// Create a parser over an existing [`Tokenizer`].
    #[must_use]
    pub const fn new(tokenizer: Tokenizer<'a>) -> Self {
        Self { tokenizer }
    }

    /// Convenience constructor from a UTF-8 string slice.
    #[must_use]
    pub const fn from_str(input: &'a str) -> Self {
        Self::new(Tokenizer::from_str(input))
    }

    /// Borrow the underlying [`Tokenizer`].
    #[must_use]
    pub const fn tokenizer(&self) -> &Tokenizer<'a> {
        &self.tokenizer
    }

    /// Mutably borrow the underlying [`Tokenizer`].
    pub const fn tokenizer_mut(&mut self) -> &mut Tokenizer<'a> {
        &mut self.tokenizer
    }

    /// Parse a single [`Value`] (number, string, boolean, `[ ... ]` array, or `{ ... }` hash).
    ///
    /// # Errors
    /// Returns [`ParseError`] if an unexpected token or lexical error is encountered.
    pub fn read_value(&mut self) -> Result<Value, ParseError> {
        let tok = self.tokenizer.next_token();
        match tok {
            TokenType::Number => {
                let n = self
                    .tokenizer
                    .number_f64()
                    .ok_or_else(|| self.error("missing numeric value"))?;
                Ok(Value::Number(n))
            }
            TokenType::String => {
                let s = self
                    .tokenizer
                    .string_value()
                    .ok_or_else(|| self.error("missing string value"))?
                    .to_owned();
                Ok(Value::String(s))
            }
            TokenType::Name => {
                let name = self
                    .tokenizer
                    .name_value()
                    .ok_or_else(|| self.error("missing name value"))?;
                match name {
                    "true" => Ok(Value::Boolean(true)),
                    "false" => Ok(Value::Boolean(false)),
                    other => Err(self.error(format!("unexpected identifier '{other}'"))),
                }
            }
            TokenType::BeginArray => self.read_array_body(),
            TokenType::BeginGroup => self.read_hash_body().map(Value::Hash),
            TokenType::Error => Err(self.error("lexical error")),
            other => Err(self.error(format!("unexpected token {other:?} while reading value"))),
        }
    }

    /// Parse a `{ key value ... }` block into an [`AssociativeArray`].
    ///
    /// # Errors
    /// Returns [`ParseError`] if the opening `{`, any entry, or the closing `}` is malformed.
    pub fn read_hash(&mut self) -> Result<AssociativeArray, ParseError> {
        match self.tokenizer.next_token() {
            TokenType::BeginGroup => self.read_hash_body(),
            other => Err(self.error(format!("expected '{{', found {other:?}"))),
        }
    }

    fn read_array_body(&mut self) -> Result<Value, ParseError> {
        let mut items = Vec::new();
        loop {
            let tok = self.tokenizer.next_token();
            if tok == TokenType::EndArray {
                return Ok(Value::Array(items));
            }
            if tok == TokenType::End {
                return Err(self.error("unterminated array '['"));
            }
            self.tokenizer.push_back();
            items.push(self.read_value()?);
        }
    }

    fn read_hash_body(&mut self) -> Result<AssociativeArray, ParseError> {
        let mut hash = AssociativeArray::new();
        loop {
            let tok = self.tokenizer.next_token();
            match tok {
                TokenType::EndGroup => return Ok(hash),
                TokenType::Name => {
                    let key = self
                        .tokenizer
                        .name_value()
                        .ok_or_else(|| self.error("missing key name"))?
                        .to_owned();
                    let val = self.read_value()?;
                    hash.insert(key, val);
                }
                TokenType::End => return Err(self.error("unterminated group '{'")),
                other => {
                    return Err(
                        self.error(format!("expected property name or '}}', found {other:?}"))
                    );
                }
            }
        }
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(self.tokenizer.line_number(), msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_ssc_style_hash() -> Result<(), String> {
        let src = r#"
        {
            Class "planet"
            Radius 6378.137
            Oblateness 0.00335
            Atmosphere {
                Height 60
                Lower [ 0.43 0.52 0.65 ]
                MieAsymmetry -0.85
            }
            Visible true
        }
        "#;

        let mut parser = Parser::from_str(src);
        let root = parser.read_hash().map_err(|e| e.to_string())?;
        assert_eq!(root.get_string("Class"), Some("planet"));
        assert_eq!(root.get_number("Radius"), Some(6378.137));
        assert_eq!(root.get_bool("Visible"), Some(true));

        let atmo = root
            .get("Atmosphere")
            .ok_or("missing Atmosphere")?
            .as_hash()
            .ok_or("Atmosphere not hash")?;
        assert_eq!(atmo.get_i64("Height"), Some(60));
        assert_eq!(atmo.get_vec3("Lower"), Some([0.43, 0.52, 0.65]));
        assert_eq!(atmo.get_number("MieAsymmetry"), Some(-0.85));
        Ok(())
    }

    #[test]
    fn rejects_unterminated_group_with_line_number() {
        let mut parser = Parser::from_str("{\n  Radius 100\n");
        assert_eq!(parser.read_hash().err().map(|e| e.line), Some(3));
    }
}
