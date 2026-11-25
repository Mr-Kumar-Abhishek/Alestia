//! Unicode comparison, prefix matching, lossy byte sanitization, and layout direction helpers.
//!
//! Covers core text utilities from `src/celutil/utf8.h` and `src/celutil/unicode.h`.

use std::cmp::Ordering;

/// UI and HUD text layout direction (`LayoutDirection` in `celestia.cfg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LayoutDirection {
    /// Left-to-right text layout (default).
    #[default]
    Ltr,
    /// Right-to-left text layout.
    Rtl,
}

impl LayoutDirection {
    /// Parse `"ltr"` or `"rtl"` (ASCII case-insensitive).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("ltr") {
            Some(Self::Ltr)
        } else if value.eq_ignore_ascii_case("rtl") {
            Some(Self::Rtl)
        } else {
            None
        }
    }
}

/// Compare two UTF-8 strings using Unicode case-folding (`to_lowercase`) order.
#[must_use]
pub fn compare_ignoring_case(a: &str, b: &str) -> Ordering {
    let mut ca = a.chars().flat_map(char::to_lowercase);
    let mut cb = b.chars().flat_map(char::to_lowercase);

    loop {
        match (ca.next(), cb.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => match x.cmp(&y) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            },
        }
    }
}

/// Check whether `haystack` starts with `prefix`, optionally ignoring case.
#[must_use]
pub fn utf8_starts_with(haystack: &str, prefix: &str, ignore_case: bool) -> bool {
    if !ignore_case {
        return haystack.starts_with(prefix);
    }

    let mut ch = haystack.chars().flat_map(char::to_lowercase);
    for p in prefix.chars().flat_map(char::to_lowercase) {
        if ch.next() != Some(p) {
            return false;
        }
    }
    true
}

/// Convert arbitrary bytes into a valid UTF-8 [`String`], replacing invalid sequences with `U+FFFD`.
#[must_use]
pub fn sanitize_utf8_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_insensitive_compare_and_prefix() {
        assert_eq!(compare_ignoring_case("Alpha", "alpha"), Ordering::Equal);
        assert_eq!(compare_ignoring_case("α", "Α"), Ordering::Equal);
        assert!(utf8_starts_with("Epsilon Indi", "EPS", true));
        assert!(!utf8_starts_with("Epsilon Indi", "EPS", false));
    }

    #[test]
    fn layout_direction_parsing() {
        assert_eq!(LayoutDirection::parse("ltr"), Some(LayoutDirection::Ltr));
        assert_eq!(LayoutDirection::parse("RTL"), Some(LayoutDirection::Rtl));
        assert_eq!(LayoutDirection::parse("other"), None);
    }

    #[test]
    fn sanitizes_invalid_utf8() {
        assert_eq!(sanitize_utf8_bytes(b"ok\xff!"), "ok\u{FFFD}!");
    }
}
