//! Lexical scanner for Celestia catalog and configuration files.
//!
//! Matches `celestia::util::Tokenizer` (`src/celutil/tokenizer.cpp` and `test/unit/tokenizer_test.cpp`).

/// Token categories produced by [`Tokenizer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Unquoted identifier (`[A-Za-z_][A-Za-z0-9_]*`).
    Name,
    /// Double-quoted string literal with escape sequences decoded.
    String,
    /// Integer or floating-point numeric literal.
    Number,
    /// Opening brace `{`.
    BeginGroup,
    /// Closing brace `}`.
    EndGroup,
    /// Opening bracket `[`.
    BeginArray,
    /// Closing bracket `]`.
    EndArray,
    /// Opening angle bracket `<` (unit block start).
    BeginUnits,
    /// Closing angle bracket `>` (unit block end).
    EndUnits,
    /// Vertical bar `|`.
    Bar,
    /// Equals sign `=`.
    Equals,
    /// Lexical error (malformed escape, unterminated string, invalid numeric prefix).
    Error,
    /// End of input stream.
    End,
}

/// Byte-level lexical scanner for `.ssc`, `.stc`, `.dsc`, `.cel`, and `celestia.cfg` inputs.
#[derive(Clone, Debug)]
pub struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    pushed_back: Option<TokenType>,
    last_token: Option<TokenType>,
    name_buf: String,
    string_buf: String,
    number_f64: f64,
    number_i64: Option<i64>,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer over a byte slice.
    #[must_use]
    pub const fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            pushed_back: None,
            last_token: None,
            name_buf: String::new(),
            string_buf: String::new(),
            number_f64: 0.0,
            number_i64: None,
        }
    }

    /// Create a new tokenizer from a UTF-8 string slice.
    #[must_use]
    pub const fn from_str(input: &'a str) -> Self {
        Self::new(input.as_bytes())
    }

    /// Current 1-based line number in the input.
    #[must_use]
    pub const fn line_number(&self) -> usize {
        self.line
    }

    /// Push back the most recently read token so the next [`Self::next_token`] yields it again.
    pub fn push_back(&mut self) {
        if let Some(tok) = self.last_token {
            self.pushed_back = Some(tok);
        }
    }

    /// Return the identifier string if the last token was [`TokenType::Name`].
    #[must_use]
    pub fn name_value(&self) -> Option<&str> {
        (self.last_token == Some(TokenType::Name)).then_some(self.name_buf.as_str())
    }

    /// Return the decoded string literal if the last token was [`TokenType::String`].
    #[must_use]
    pub fn string_value(&self) -> Option<&str> {
        (self.last_token == Some(TokenType::String)).then_some(self.string_buf.as_str())
    }

    /// Return the floating-point numeric value if the last token was [`TokenType::Number`].
    #[must_use]
    pub fn number_f64(&self) -> Option<f64> {
        (self.last_token == Some(TokenType::Number)).then_some(self.number_f64)
    }

    /// Return the integer numeric value if the last token was an integer [`TokenType::Number`].
    #[must_use]
    pub fn number_i64(&self) -> Option<i64> {
        if self.last_token == Some(TokenType::Number) {
            self.number_i64
        } else {
            None
        }
    }

    /// Advance to and return the next [`TokenType`].
    pub fn next_token(&mut self) -> TokenType {
        if let Some(pushed) = self.pushed_back.take() {
            self.last_token = Some(pushed);
            return pushed;
        }

        self.skip_whitespace_and_comments();

        let Some(&ch) = self.input.get(self.pos) else {
            self.last_token = Some(TokenType::End);
            return TokenType::End;
        };

        let tok = match ch {
            b'{' => {
                self.pos += 1;
                TokenType::BeginGroup
            }
            b'}' => {
                self.pos += 1;
                TokenType::EndGroup
            }
            b'[' => {
                self.pos += 1;
                TokenType::BeginArray
            }
            b']' => {
                self.pos += 1;
                TokenType::EndArray
            }
            b'<' => {
                self.pos += 1;
                TokenType::BeginUnits
            }
            b'>' => {
                self.pos += 1;
                TokenType::EndUnits
            }
            b'|' => {
                self.pos += 1;
                TokenType::Bar
            }
            b'=' => {
                self.pos += 1;
                TokenType::Equals
            }
            b'"' => self.read_string(),
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number(),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.read_name(),
            _ => {
                self.pos += 1;
                TokenType::Error
            }
        };

        self.last_token = Some(tok);
        tok
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(&b) = self.input.get(self.pos) {
            if b == b'#' {
                self.pos += 1;
                while let Some(&c) = self.input.get(self.pos) {
                    self.pos += 1;
                    if c == b'\n' {
                        self.line += 1;
                        break;
                    }
                }
            } else if b.is_ascii_whitespace() {
                if b == b'\n' {
                    self.line += 1;
                }
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn read_name(&mut self) -> TokenType {
        let start = self.pos;
        while let Some(&b) = self.input.get(self.pos) {
            if b.is_ascii_alphanumeric() || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.name_buf.clear();
        if let Ok(s) = std::str::from_utf8(&self.input[start..self.pos]) {
            self.name_buf.push_str(s);
            TokenType::Name
        } else {
            TokenType::Error
        }
    }

    fn read_string(&mut self) -> TokenType {
        self.pos += 1; // skip opening '"'
        self.string_buf.clear();

        while let Some(&b) = self.input.get(self.pos) {
            match b {
                b'"' => {
                    self.pos += 1;
                    return TokenType::String;
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(&esc) = self.input.get(self.pos) else {
                        return TokenType::Error;
                    };
                    self.pos += 1;
                    match esc {
                        b'\\' => self.string_buf.push('\\'),
                        b'n' => self.string_buf.push('\n'),
                        b'r' => self.string_buf.push('\r'),
                        b't' => self.string_buf.push('\t'),
                        b'"' => self.string_buf.push('"'),
                        b'u' => {
                            let Some(ch) = self.read_unicode_escape() else {
                                return TokenType::Error;
                            };
                            self.string_buf.push(ch);
                        }
                        _ => return TokenType::Error,
                    }
                }
                b'\n' => {
                    self.line += 1;
                    self.pos += 1;
                    self.string_buf.push('\n');
                }
                0x00..=0x7F => {
                    self.pos += 1;
                    self.string_buf.push(char::from(b));
                }
                _ => {
                    let ch = self.read_utf8_char_lossy();
                    self.string_buf.push(ch);
                }
            }
        }

        TokenType::Error
    }

    fn read_unicode_escape(&mut self) -> Option<char> {
        let hex_bytes = self.input.get(self.pos..self.pos + 4)?;
        let hex_str = std::str::from_utf8(hex_bytes).ok()?;
        let code = u32::from_str_radix(hex_str, 16).ok()?;
        self.pos += 4;
        Some(char::from_u32(code).unwrap_or('\u{FFFD}'))
    }

    /// Decode one UTF-8 scalar value matching Celestia's `UTF8Decode` behavior for invalid sequences.
    fn read_utf8_char_lossy(&mut self) -> char {
        let b0 = self.input[self.pos];
        self.pos += 1;

        let (expected_len, min_cp, mut cp) = match b0 {
            0xC2..=0xDF => (2, 0x80_u32, u32::from(b0 & 0x1F)),
            0xE0..=0xEF => (3, 0x800_u32, u32::from(b0 & 0x0F)),
            0xF0..=0xF4 => (4, 0x1_0000_u32, u32::from(b0 & 0x07)),
            _ => return '\u{FFFD}',
        };

        for _ in 1..expected_len {
            let Some(&cont) = self.input.get(self.pos) else {
                return '\u{FFFD}';
            };
            if cont == b'"' {
                return '\u{FFFD}';
            }
            self.pos += 1;
            if (cont & 0xC0) != 0x80 {
                return '\u{FFFD}';
            }
            cp = (cp << 6) | u32::from(cont & 0x3F);
        }

        if cp < min_cp {
            return '\u{FFFD}';
        }
        char::from_u32(cp).unwrap_or('\u{FFFD}')
    }

    fn read_number(&mut self) -> TokenType {
        let start = self.pos;

        if matches!(self.input.get(self.pos), Some(b'+' | b'-')) {
            self.pos += 1;
        }

        let mut has_digits = false;
        while let Some(&b) = self.input.get(self.pos) {
            if b.is_ascii_digit() {
                has_digits = true;
                self.pos += 1;
            } else {
                break;
            }
        }

        let mut is_float = false;
        if self.input.get(self.pos) == Some(&b'.') {
            is_float = true;
            self.pos += 1;
            while let Some(&b) = self.input.get(self.pos) {
                if b.is_ascii_digit() {
                    has_digits = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        if !has_digits {
            return TokenType::Error;
        }

        // Only consume exponent `[eE][+-]?[0-9]+` if at least one exponent digit exists.
        if matches!(self.input.get(self.pos), Some(b'e' | b'E')) {
            let mut exp_pos = self.pos + 1;
            if matches!(self.input.get(exp_pos), Some(b'+' | b'-')) {
                exp_pos += 1;
            }
            let digit_start = exp_pos;
            while let Some(&b) = self.input.get(exp_pos) {
                if b.is_ascii_digit() {
                    exp_pos += 1;
                } else {
                    break;
                }
            }
            if exp_pos > digit_start {
                is_float = true;
                self.pos = exp_pos;
            }
        }

        let Ok(slice) = std::str::from_utf8(&self.input[start..self.pos]) else {
            return TokenType::Error;
        };

        let Ok(val) = slice.parse::<f64>() else {
            return TokenType::Error;
        };
        self.number_f64 = val;
        self.number_i64 = if is_float {
            None
        } else {
            slice.parse::<i64>().ok()
        };
        TokenType::Number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_units() {
        let mut tok = Tokenizer::from_str("Normal Number2 Number3Number snake_case _prefixed");
        for expected in [
            "Normal",
            "Number2",
            "Number3Number",
            "snake_case",
            "_prefixed",
        ] {
            assert_eq!(tok.next_token(), TokenType::Name);
            assert_eq!(tok.name_value(), Some(expected));
        }
        assert_eq!(tok.next_token(), TokenType::End);

        let mut unit_tok = Tokenizer::from_str("Quantity<unit>");
        assert_eq!(unit_tok.next_token(), TokenType::Name);
        assert_eq!(unit_tok.name_value(), Some("Quantity"));
        assert_eq!(unit_tok.next_token(), TokenType::BeginUnits);
        assert_eq!(unit_tok.next_token(), TokenType::Name);
        assert_eq!(unit_tok.name_value(), Some("unit"));
        assert_eq!(unit_tok.next_token(), TokenType::EndUnits);
        assert_eq!(unit_tok.next_token(), TokenType::End);
    }

    #[test]
    fn parses_strings_escapes_and_utf8() {
        let mut tok = Tokenizer::from_str(r#""abc 123.456 {}<>" """#);
        assert_eq!(tok.next_token(), TokenType::String);
        assert_eq!(tok.string_value(), Some("abc 123.456 {}<>"));
        assert_eq!(tok.next_token(), TokenType::String);
        assert_eq!(tok.string_value(), Some(""));
        assert_eq!(tok.next_token(), TokenType::End);

        let mut esc = Tokenizer::from_str(r#""abc\\def\nghi\"jkl""#);
        assert_eq!(esc.next_token(), TokenType::String);
        assert_eq!(esc.string_value(), Some("abc\\def\nghi\"jkl"));
        assert_eq!(esc.next_token(), TokenType::End);

        let mut uni = Tokenizer::from_str(r#""\u00ef" "\u0900" "\udabc""#);
        assert_eq!(uni.next_token(), TokenType::String);
        assert_eq!(uni.string_value(), Some("\u{00EF}"));
        assert_eq!(uni.next_token(), TokenType::String);
        assert_eq!(uni.string_value(), Some("\u{0900}"));
        assert_eq!(uni.next_token(), TokenType::String);
        assert_eq!(uni.string_value(), Some("\u{FFFD}"));
        assert_eq!(uni.next_token(), TokenType::End);

        let mut bad_esc = Tokenizer::from_str(r#""abcdefghijklmnop\qrstuvwxyz""#);
        assert_eq!(bad_esc.next_token(), TokenType::Error);

        // Matches UTF-8 byte sequence cases from tokenizer_test.cpp
        let raw_bytes: &[u8] = b"\"\xc3\xaf\xe0\xa4\x80\" \"\xc0\" \"\xc3x\" \"\xe0\xa0x\" \"\xe0x\xb0\" \"\xc3\xaf\xe0\xa0x\"";
        let mut utf8_tok = Tokenizer::new(raw_bytes);
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{00EF}\u{0900}"));
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{FFFD}"));
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{FFFD}"));
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{FFFD}"));
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{FFFD}\u{FFFD}"));
        assert_eq!(utf8_tok.next_token(), TokenType::String);
        assert_eq!(utf8_tok.string_value(), Some("\u{00EF}\u{FFFD}"));
        assert_eq!(utf8_tok.next_token(), TokenType::End);
    }

    #[test]
    fn parses_numbers_signs_and_trailing_exponents() -> Result<(), String> {
        let mut tok = Tokenizer::from_str(
            "0 0.0 12345 12345.0 32.75 1.2e6 2.3e+6 7.5e-1 1.2E6 2.3E+6 7.5E-1",
        );
        let expected = [
            (0.0, Some(0)),
            (0.0, None),
            (12345.0, Some(12345)),
            (12345.0, None),
            (32.75, None),
            (1_200_000.0, None),
            (2_300_000.0, None),
            (0.75, None),
            (1_200_000.0, None),
            (2_300_000.0, None),
            (0.75, None),
        ];
        for (fval, ival) in expected {
            assert_eq!(tok.next_token(), TokenType::Number);
            assert_eq!(tok.number_f64(), Some(fval));
            assert_eq!(tok.number_i64(), ival);
        }
        assert_eq!(tok.next_token(), TokenType::End);

        let mut neg = Tokenizer::from_str("-0 -0.0 -12345 -12345.0 -32.75 -1.2e6 -2.3e+6 -7.5e-1");
        assert_eq!(neg.next_token(), TokenType::Number);
        let n0 = neg.number_f64().ok_or("missing -0")?;
        assert_eq!(n0, -0.0);
        assert!(n0.is_sign_negative());
        assert_eq!(neg.number_i64(), Some(0));

        assert_eq!(neg.next_token(), TokenType::Number);
        let n0f = neg.number_f64().ok_or("missing -0.0")?;
        assert_eq!(n0f, -0.0);
        assert!(n0f.is_sign_negative());
        assert_eq!(neg.number_i64(), None);

        for invalid in ["+", "-", "+e", "+E", "-e", "-E"] {
            let mut bad = Tokenizer::from_str(invalid);
            assert_eq!(bad.next_token(), TokenType::Error, "input={invalid}");
        }

        for trailing in ["1.25e", "1.25E", "1.25e+", "1.25e-", "1.25E+", "1.25E-"] {
            let mut tr = Tokenizer::from_str(trailing);
            assert_eq!(tr.next_token(), TokenType::Number);
            assert_eq!(tr.number_f64(), Some(1.25));
            assert_eq!(tr.number_i64(), None);
            assert_eq!(tr.next_token(), TokenType::Name);
            assert_eq!(tr.name_value(), Some(&trailing[4..5]));
        }

        let mut sep = Tokenizer::from_str("123{");
        assert_eq!(sep.next_token(), TokenType::Number);
        assert_eq!(sep.number_i64(), Some(123));
        assert_eq!(sep.next_token(), TokenType::BeginGroup);
        assert_eq!(sep.next_token(), TokenType::End);
        Ok(())
    }

    #[test]
    fn parses_symbols_comments_and_push_back() {
        let mut sym = Tokenizer::from_str("={}|[]<>");
        for expected in [
            TokenType::Equals,
            TokenType::BeginGroup,
            TokenType::EndGroup,
            TokenType::Bar,
            TokenType::BeginArray,
            TokenType::EndArray,
            TokenType::BeginUnits,
            TokenType::EndUnits,
            TokenType::End,
        ] {
            assert_eq!(sym.next_token(), expected);
        }

        let comment_bytes: &[u8] = b"Token1 # comment\nToken2 # \xc0\nToken3 # blah";
        let mut com = Tokenizer::new(comment_bytes);
        assert_eq!(com.next_token(), TokenType::Name);
        assert_eq!(com.name_value(), Some("Token1"));
        com.push_back();
        assert_eq!(com.next_token(), TokenType::Name);
        assert_eq!(com.name_value(), Some("Token1"));
        assert_eq!(com.next_token(), TokenType::Name);
        assert_eq!(com.name_value(), Some("Token2"));
        assert_eq!(com.next_token(), TokenType::Name);
        assert_eq!(com.name_value(), Some("Token3"));
        assert_eq!(com.line_number(), 3);
        assert_eq!(com.next_token(), TokenType::End);
    }
}
