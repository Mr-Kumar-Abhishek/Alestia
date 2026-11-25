//! Natural string comparison matching Celestia's `strnatcmp` (`src/celutil/strnatcmp.cpp`).
//!
//! Compares embedded numeric runs by numerical magnitude (unless zero-prefixed, as in fractional
//! parts such as `"1.05"` < `"1.2"`), while ignoring leading whitespace.

use std::cmp::Ordering;

/// Compare two strings using natural ordering (case-sensitive).
#[must_use]
pub fn strnatcmp(a: &str, b: &str) -> Ordering {
    compare_nat(a.as_bytes(), b.as_bytes(), false)
}

/// Compare two strings using natural ordering (ASCII case-insensitive).
#[must_use]
pub fn strnatcasecmp(a: &str, b: &str) -> Ordering {
    compare_nat(a.as_bytes(), b.as_bytes(), true)
}

fn compare_nat(a: &[u8], b: &[u8], fold_case: bool) -> Ordering {
    let mut ai = 0_usize;
    let mut bi = 0_usize;

    loop {
        while ai < a.len() && a[ai].is_ascii_whitespace() {
            ai += 1;
        }
        while bi < b.len() && b[bi].is_ascii_whitespace() {
            bi += 1;
        }

        let ca = a.get(ai).copied();
        let cb = b.get(bi).copied();

        match (ca, cb) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(da), Some(db)) => {
                if da.is_ascii_digit() && db.is_ascii_digit() {
                    let fractional = da == b'0' || db == b'0';
                    let (ord, adv_a, adv_b) = if fractional {
                        compare_left(&a[ai..], &b[bi..])
                    } else {
                        compare_right(&a[ai..], &b[bi..])
                    };
                    if ord != Ordering::Equal {
                        return ord;
                    }
                    ai += adv_a;
                    bi += adv_b;
                    continue;
                }

                let fa = if fold_case {
                    da.to_ascii_uppercase()
                } else {
                    da
                };
                let fb = if fold_case {
                    db.to_ascii_uppercase()
                } else {
                    db
                };
                match fa.cmp(&fb) {
                    Ordering::Equal => {
                        ai += 1;
                        bi += 1;
                    }
                    non_eq => return non_eq,
                }
            }
        }
    }
}

fn compare_left(a: &[u8], b: &[u8]) -> (Ordering, usize, usize) {
    let mut i = 0_usize;
    loop {
        let da = a.get(i).copied().filter(u8::is_ascii_digit);
        let db = b.get(i).copied().filter(u8::is_ascii_digit);
        match (da, db) {
            (None, None) => return (Ordering::Equal, i, i),
            (None, Some(_)) => return (Ordering::Less, i, i),
            (Some(_), None) => return (Ordering::Greater, i, i),
            (Some(ca), Some(cb)) => match ca.cmp(&cb) {
                Ordering::Equal => i += 1,
                non_eq => return (non_eq, i, i),
            },
        }
    }
}

fn compare_right(a: &[u8], b: &[u8]) -> (Ordering, usize, usize) {
    let mut bias = Ordering::Equal;
    let mut i = 0_usize;
    loop {
        let da = a.get(i).copied().filter(u8::is_ascii_digit);
        let db = b.get(i).copied().filter(u8::is_ascii_digit);
        match (da, db) {
            (None, None) => return (bias, i, i),
            (None, Some(_)) => return (Ordering::Less, i, i),
            (Some(_), None) => return (Ordering::Greater, i, i),
            (Some(ca), Some(cb)) => {
                if bias == Ordering::Equal {
                    bias = ca.cmp(&cb);
                }
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_numbers() {
        let examples = [
            ("abc", "abc", Ordering::Equal),
            ("de", "def", Ordering::Less),
            ("ghi", "gh", Ordering::Greater),
            ("abc", "def", Ordering::Less),
            ("ab", "def", Ordering::Less),
            ("abc", "de", Ordering::Less),
            ("jkl", "ghi", Ordering::Greater),
            ("jk", "ghi", Ordering::Greater),
            ("jkl", "gh", Ordering::Greater),
        ];
        for (a, b, expected) in examples {
            assert_eq!(strnatcmp(a, b), expected, "{a} vs {b}");
        }
    }

    #[test]
    fn with_numbers_and_decimals() {
        let examples = [
            ("123", "123", Ordering::Equal),
            ("1.23", "1.23", Ordering::Equal),
            ("12", "123", Ordering::Less),
            ("89", "123", Ordering::Less),
            ("123", "12", Ordering::Greater),
            ("123", "89", Ordering::Greater),
            ("1.05", "1.2", Ordering::Less),
            ("1.2", "1.05", Ordering::Greater),
            ("z123", "z123", Ordering::Equal),
            ("z1.23", "z1.23", Ordering::Equal),
            ("z12", "z123", Ordering::Less),
            ("z89", "z123", Ordering::Less),
            ("z123", "z12", Ordering::Greater),
            ("z123", "z89", Ordering::Greater),
            ("z1.05", "z1.2", Ordering::Less),
            ("z1.2", "z1.05", Ordering::Greater),
            ("123z", "123z", Ordering::Equal),
            ("1.23z", "1.23z", Ordering::Equal),
            ("12z", "123z", Ordering::Less),
            ("89z", "123z", Ordering::Less),
            ("123z", "12z", Ordering::Greater),
            ("123z", "89z", Ordering::Greater),
            ("1.05z", "1.2z", Ordering::Less),
            ("1.2z", "1.05z", Ordering::Greater),
            ("x123z", "x123z", Ordering::Equal),
            ("x1.23z", "x1.23z", Ordering::Equal),
            ("x12z", "x123z", Ordering::Less),
            ("x89z", "x123z", Ordering::Less),
            ("x123z", "x12z", Ordering::Greater),
            ("x123z", "x89z", Ordering::Greater),
            ("x1.05z", "x1.2z", Ordering::Less),
            ("x1.2z", "x1.05z", Ordering::Greater),
            ("987abc", "987abc", Ordering::Equal),
            ("987de", "987def", Ordering::Less),
            ("987ghi", "987gh", Ordering::Greater),
            ("987abc", "987def", Ordering::Less),
            ("987ab", "987def", Ordering::Less),
            ("987abc", "987de", Ordering::Less),
            ("987jkl", "987ghi", Ordering::Greater),
            ("987jk", "987ghi", Ordering::Greater),
            ("987jkl", "987gh", Ordering::Greater),
            ("abc987", "abc987", Ordering::Equal),
            ("de987", "def987", Ordering::Less),
            ("ghi987", "gh987", Ordering::Greater),
            ("abc987", "def987", Ordering::Less),
            ("ab987", "def987", Ordering::Less),
            ("abc987", "de987", Ordering::Less),
            ("jkl987", "ghi987", Ordering::Greater),
            ("jk987", "ghi987", Ordering::Greater),
            ("jkl987", "gh987", Ordering::Greater),
        ];
        for (a, b, expected) in examples {
            assert_eq!(strnatcmp(a, b), expected, "{a} vs {b}");
        }
    }

    #[test]
    fn skips_leading_whitespace() {
        let examples = [
            ("   abc", "abc", Ordering::Equal),
            ("   de", "def", Ordering::Less),
            ("   ghi", "gh", Ordering::Greater),
            ("   abc", "def", Ordering::Less),
            ("   ab", "def", Ordering::Less),
            ("   abc", "de", Ordering::Less),
            ("   jkl", "ghi", Ordering::Greater),
            ("   jk", "ghi", Ordering::Greater),
            ("   jkl", "gh", Ordering::Greater),
            ("abc", "   abc", Ordering::Equal),
            ("de", "   def", Ordering::Less),
            ("ghi", "   gh", Ordering::Greater),
            ("abc", "   def", Ordering::Less),
            ("ab", "   def", Ordering::Less),
            ("abc", "   de", Ordering::Less),
            ("jkl", "   ghi", Ordering::Greater),
            ("jk", "   ghi", Ordering::Greater),
            ("jkl", "   gh", Ordering::Greater),
        ];
        for (a, b, expected) in examples {
            assert_eq!(strnatcmp(a, b), expected, "{a} vs {b}");
        }
    }

    #[test]
    fn case_insensitive_comparison() {
        assert_eq!(strnatcasecmp("hip 2", "HIP 10"), Ordering::Less);
        assert_eq!(strnatcasecmp("Alpha", "alpha"), Ordering::Equal);
    }
}
