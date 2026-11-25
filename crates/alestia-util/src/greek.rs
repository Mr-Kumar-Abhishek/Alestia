//! Bayer Greek-letter abbreviation expansion and UTF-8 superscript formatting.
//!
//! Matches `src/celutil/greek.cpp` and `test/unit/greek_test.cpp`.

struct GreekEntry {
    name: &'static str,
    abbr: &'static str,
    utf8: &'static str,
}

const GREEK_LETTERS: [GreekEntry; 24] = [
    GreekEntry {
        name: "Alpha",
        abbr: "ALF",
        utf8: "\u{03B1}",
    },
    GreekEntry {
        name: "Beta",
        abbr: "BET",
        utf8: "\u{03B2}",
    },
    GreekEntry {
        name: "Gamma",
        abbr: "GAM",
        utf8: "\u{03B3}",
    },
    GreekEntry {
        name: "Delta",
        abbr: "DEL",
        utf8: "\u{03B4}",
    },
    GreekEntry {
        name: "Epsilon",
        abbr: "EPS",
        utf8: "\u{03B5}",
    },
    GreekEntry {
        name: "Zeta",
        abbr: "ZET",
        utf8: "\u{03B6}",
    },
    GreekEntry {
        name: "Eta",
        abbr: "ETA",
        utf8: "\u{03B7}",
    },
    GreekEntry {
        name: "Theta",
        abbr: "TET",
        utf8: "\u{03B8}",
    },
    GreekEntry {
        name: "Iota",
        abbr: "IOT",
        utf8: "\u{03B9}",
    },
    GreekEntry {
        name: "Kappa",
        abbr: "KAP",
        utf8: "\u{03BA}",
    },
    GreekEntry {
        name: "Lambda",
        abbr: "LAM",
        utf8: "\u{03BB}",
    },
    GreekEntry {
        name: "Mu",
        abbr: "MU",
        utf8: "\u{03BC}",
    },
    GreekEntry {
        name: "Nu",
        abbr: "NU",
        utf8: "\u{03BD}",
    },
    GreekEntry {
        name: "Xi",
        abbr: "XI",
        utf8: "\u{03BE}",
    },
    GreekEntry {
        name: "Omicron",
        abbr: "OMI",
        utf8: "\u{03BF}",
    },
    GreekEntry {
        name: "Pi",
        abbr: "PI",
        utf8: "\u{03C0}",
    },
    GreekEntry {
        name: "Rho",
        abbr: "RHO",
        utf8: "\u{03C1}",
    },
    GreekEntry {
        name: "Sigma",
        abbr: "SIG",
        utf8: "\u{03C3}",
    },
    GreekEntry {
        name: "Tau",
        abbr: "TAU",
        utf8: "\u{03C4}",
    },
    GreekEntry {
        name: "Upsilon",
        abbr: "UPS",
        utf8: "\u{03C5}",
    },
    GreekEntry {
        name: "Phi",
        abbr: "PHI",
        utf8: "\u{03C6}",
    },
    GreekEntry {
        name: "Chi",
        abbr: "CHI",
        utf8: "\u{03C7}",
    },
    GreekEntry {
        name: "Psi",
        abbr: "PSI",
        utf8: "\u{03C8}",
    },
    GreekEntry {
        name: "Omega",
        abbr: "OME",
        utf8: "\u{03C9}",
    },
];

/// Replace a leading uppercase Greek letter name/abbreviation with its UTF-8 Greek glyph
/// and convert trailing numeric qualifiers into UTF-8 superscripts.
///
/// Matches `ReplaceGreekLetterAbbr`: only triggers when the first byte is in `'A'..='Z'`.
#[must_use]
pub fn replace_greek_letter_abbr(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let first_byte = input.as_bytes()[0];
    if !first_byte.is_ascii_uppercase() {
        return input.to_owned();
    }

    let len = first_word_length(input);
    if len == 0 {
        return input.to_owned();
    }

    let word = &input[..len];
    for entry in &GREEK_LETTERS {
        if word.eq_ignore_ascii_case(entry.abbr) || word.eq_ignore_ascii_case(entry.name) {
            return format_with_superscripts(entry.utf8, &input[len..]);
        }
    }

    input.to_owned()
}

/// Replace a leading Greek letter name, abbreviation, or UTF-8 glyph (case-insensitive)
/// with its UTF-8 Greek glyph and convert trailing numeric qualifiers into UTF-8 superscripts.
#[must_use]
pub fn replace_greek_letter(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let len = first_word_length(input);
    if len == 0 {
        return input.to_owned();
    }

    let word = &input[..len];
    for entry in &GREEK_LETTERS {
        if word == entry.utf8
            || word.eq_ignore_ascii_case(entry.abbr)
            || word.eq_ignore_ascii_case(entry.name)
        {
            return format_with_superscripts(entry.utf8, &input[len..]);
        }
    }

    input.to_owned()
}

/// Return the canonical 3-letter Bayer abbreviation (e.g. `"ALF"`, `"XI"`) for a Latin name,
/// abbreviation, or UTF-8 Greek letter.
#[must_use]
pub fn canonical_greek_abbreviation(letter: &str) -> Option<&'static str> {
    for entry in &GREEK_LETTERS {
        if letter.eq_ignore_ascii_case(entry.name)
            || letter.eq_ignore_ascii_case(entry.abbr)
            || letter == entry.utf8
        {
            return Some(entry.abbr);
        }
    }
    None
}

fn first_word_length(s: &str) -> usize {
    let mut end = s.find(' ').unwrap_or(s.len());
    let bytes = s.as_bytes();
    while end > 0 && bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    end
}

fn format_with_superscripts(greek_utf8: &str, rest: &str) -> String {
    let mut out = String::from(greek_utf8);
    let mut idx = 0_usize;
    let bytes = rest.as_bytes();
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        if let Some(sup) = to_superscript(bytes[idx]) {
            out.push_str(sup);
        }
        idx += 1;
    }
    out.push_str(&rest[idx..]);
    out
}

const fn to_superscript(digit: u8) -> Option<&'static str> {
    match digit {
        b'0' => Some("\u{2070}"),
        b'1' => Some("\u{00B9}"),
        b'2' => Some("\u{00B2}"),
        b'3' => Some("\u{00B3}"),
        b'4' => Some("\u{2074}"),
        b'5' => Some("\u{2075}"),
        b'6' => Some("\u{2076}"),
        b'7' => Some("\u{2077}"),
        b'8' => Some("\u{2078}"),
        b'9' => Some("\u{2079}"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_greek_letter_abbr_cases() {
        assert_eq!(replace_greek_letter_abbr("XI"), "\u{03BE}");
        assert_eq!(
            replace_greek_letter_abbr("XI12"),
            "\u{03BE}\u{00B9}\u{00B2}"
        );
        assert_eq!(replace_greek_letter_abbr("XI Foo"), "\u{03BE} Foo");
        assert_eq!(
            replace_greek_letter_abbr("XI12 Bar"),
            "\u{03BE}\u{00B9}\u{00B2} Bar"
        );
        assert_eq!(replace_greek_letter_abbr("xi"), "xi");
        assert_eq!(
            replace_greek_letter_abbr("Alpha Centauri"),
            "\u{03B1} Centauri"
        );
    }

    #[test]
    fn replace_greek_letter_cases() {
        assert_eq!(replace_greek_letter("xi"), "\u{03BE}");
        assert_eq!(
            replace_greek_letter("xi12 Bar"),
            "\u{03BE}\u{00B9}\u{00B2} Bar"
        );
        assert_eq!(replace_greek_letter("alpha"), "\u{03B1}");
        assert_eq!(replace_greek_letter("\u{03BE}1"), "\u{03BE}\u{00B9}");
    }

    #[test]
    fn canonical_abbreviation_cases() {
        assert_eq!(canonical_greek_abbreviation("xi"), Some("XI"));
        assert_eq!(canonical_greek_abbreviation("alpha"), Some("ALF"));
        assert_eq!(canonical_greek_abbreviation("\u{03B1}"), Some("ALF"));
        assert_eq!(canonical_greek_abbreviation("unknown"), None);
    }
}
