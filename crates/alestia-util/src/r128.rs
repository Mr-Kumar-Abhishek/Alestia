//! Signed 128-bit (`64.64`) fixed-point arithmetic and URL-safe Base64 serialization.
//!
//! Matches Celestia's `R128` coordinate representation (`src/celutil/r128.h` and `r128util.cpp`),
//! used to represent universal coordinates in micro-light-years without floating-point jitter.

use std::fmt;

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const MAX_ENCODED_LEN: usize = 22; // (16 * 4 + 2) / 3
const FRAC_SCALE: f64 = 18_446_744_073_709_551_616.0; // 2^64

/// Signed 128-bit fixed-point number in `64.64` format.
///
/// The upper 64 bits (`hi`) store the signed integer part and the lower 64 bits (`lo`) store the
/// fractional part in units of $2^{-64}$.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct R128 {
    raw: i128,
}

impl R128 {
    /// Additive identity (`0.0`).
    pub const ZERO: Self = Self { raw: 0 };

    /// Maximum representable signed `64.64` value.
    pub const MAX: Self = Self { raw: i128::MAX };

    /// Minimum representable signed `64.64` value.
    pub const MIN: Self = Self { raw: i128::MIN };

    /// Construct from raw signed 128-bit integer representation.
    #[must_use]
    pub const fn from_raw(raw: i128) -> Self {
        Self { raw }
    }

    /// Return the underlying signed 128-bit integer representation.
    #[must_use]
    pub const fn to_raw(self) -> i128 {
        self.raw
    }

    /// Construct from Celestia's `(lo, hi)` 64-bit little-endian limb pair.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn from_parts(lo: u64, hi: u64) -> Self {
        let raw = ((hi as i128) << 64) | (lo as i128);
        Self { raw }
    }

    /// Return the lower 64 fractional bits.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn lo(self) -> u64 {
        (self.raw as u128) as u64
    }

    /// Return the upper 64 signed integer bits as a `u64` bit pattern.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn hi(self) -> u64 {
        ((self.raw as u128) >> 64) as u64
    }

    /// Convert an `f64` into `64.64` fixed-point representation.
    ///
    /// Non-finite inputs return [`R128::ZERO`]; out-of-range finite values saturate at
    /// [`R128::MIN`] or [`R128::MAX`].
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return Self::ZERO;
        }
        let int_part = value.trunc();
        if int_part >= (i64::MAX as f64) {
            return Self::MAX;
        }
        if int_part <= (i64::MIN as f64) {
            return Self::MIN;
        }
        let frac_part = value - int_part;
        let hi = (int_part as i128) << 64;
        let lo = (frac_part * FRAC_SCALE).round() as i128;
        Self {
            raw: hi.saturating_add(lo),
        }
    }

    /// Convert `64.64` fixed-point value to `f64`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_f64(self) -> f64 {
        let hi = (self.raw >> 64) as f64;
        let lo = (self.lo() as f64) / FRAC_SCALE;
        hi + lo
    }

    /// Checked addition returning `None` on signed 128-bit overflow.
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.raw.checked_add(rhs.raw) {
            Some(raw) => Some(Self { raw }),
            None => None,
        }
    }

    /// Checked subtraction returning `None` on signed 128-bit overflow.
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.raw.checked_sub(rhs.raw) {
            Some(raw) => Some(Self { raw }),
            None => None,
        }
    }

    /// Checked negation returning `None` for [`R128::MIN`].
    #[must_use]
    pub const fn checked_neg(self) -> Option<Self> {
        match self.raw.checked_neg() {
            Some(raw) => Some(Self { raw }),
            None => None,
        }
    }

    /// Checked `64.64` fixed-point multiplication, returning `None` on overflow.
    ///
    /// Computes `(self * rhs) >> 64` using exact 256-bit intermediate arithmetic.
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        let neg = (self.raw < 0) ^ (rhs.raw < 0);
        let a = self.raw.unsigned_abs();
        let b = rhs.raw.unsigned_abs();

        let a_lo = a & 0xFFFF_FFFF_FFFF_FFFF;
        let a_hi = a >> 64;
        let b_lo = b & 0xFFFF_FFFF_FFFF_FFFF;
        let b_hi = b >> 64;

        let p00 = a_lo * b_lo;
        let p01 = a_lo * b_hi;
        let p10 = a_hi * b_lo;
        let p11 = a_hi * b_hi;

        let mid = (p00 >> 64) + (p01 & 0xFFFF_FFFF_FFFF_FFFF) + (p10 & 0xFFFF_FFFF_FFFF_FFFF);
        let lo64 = (p00 & 0xFFFF_FFFF_FFFF_FFFF) | ((mid & 0xFFFF_FFFF_FFFF_FFFF) << 64);
        let carry_to_mid_hi = mid >> 64;
        let Some(hi_s1) = p11.checked_add(p01 >> 64) else {
            return None;
        };
        let Some(hi_s2) = hi_s1.checked_add(p10 >> 64) else {
            return None;
        };
        let Some(hi128) = hi_s2.checked_add(carry_to_mid_hi) else {
            return None;
        };

        // Shift right by 64 bits: result_u128 = (hi128 << 64) | (lo64 >> 64).
        if hi128 >> 64 != 0 {
            return None;
        }
        let mag = (hi128 << 64) | (lo64 >> 64);
        if neg {
            if mag > (i128::MAX as u128) + 1 {
                return None;
            }
            if mag == (i128::MAX as u128) + 1 {
                Some(Self::MIN)
            } else {
                Some(Self {
                    raw: -(mag as i128),
                })
            }
        } else if mag > (i128::MAX as u128) {
            None
        } else {
            Some(Self { raw: mag as i128 })
        }
    }

    /// Check Celestia's universal-coordinate bounds predicate (`celestia::util::isOutOfBounds`).
    #[must_use]
    pub const fn is_out_of_bounds(self) -> bool {
        const HI_THRESHOLD: u64 = 1_u64 << 62;
        const LO_THRESHOLD: u64 = HI_THRESHOLD.wrapping_neg();
        let hi = self.hi();
        hi > HI_THRESHOLD && hi < LO_THRESHOLD
    }

    /// Encode this `R128` into Celestia's compact unpadded Base64 representation.
    ///
    /// Matches `fmt::formatter<R128>` in `src/celutil/r128util.cpp`: `R128(0, 0)` encodes to `""`
    /// and `R128(u64::MAX, u64::MAX)` encodes to `"/////////////////////w"`.
    #[must_use]
    pub fn encode_base64(self) -> String {
        let lo = self.lo();
        let hi = self.hi();
        let byte_count: usize = if hi == 0 {
            8 - (lo.leading_zeros() as usize >> 3)
        } else {
            16 - (hi.leading_zeros() as usize >> 3)
        };

        let mut encoded = String::with_capacity(MAX_ENCODED_LEN);
        let mut output: u32 = 0;
        let mut group_idx: usize = 0;

        for i in 0..byte_count {
            let limb = if i < 8 { lo } else { hi };
            let byte = ((limb >> ((i & 7) << 3)) & 0xFF) as u32;
            output |= byte;
            group_idx += 1;
            if group_idx == 3 {
                push_b64_char(&mut encoded, output >> 18);
                push_b64_char(&mut encoded, (output >> 12) & 0x3F);
                push_b64_char(&mut encoded, (output >> 6) & 0x3F);
                push_b64_char(&mut encoded, output & 0x3F);
                output = 0;
                group_idx = 0;
            } else {
                output <<= 8;
            }
        }

        if group_idx > 0 {
            output <<= 16 - (group_idx << 3);
            push_b64_char(&mut encoded, output >> 18);
            push_b64_char(&mut encoded, (output >> 12) & 0x3F);
            if group_idx > 1 {
                push_b64_char(&mut encoded, (output >> 6) & 0x3F);
            }
        }

        encoded
    }

    /// Decode Celestia's compact Base64 representation into an [`R128`].
    ///
    /// Matches `celestia::util::DecodeFromBase64` in `src/celutil/r128util.cpp`:
    /// ignores trailing `'='` padding, decodes `""` as `R128::ZERO`, rejects strings longer
    /// than 22 characters or with `len % 4 == 1`, rejects non-alphabet characters, and rejects
    /// non-zero unused trailing padding bits.
    #[must_use]
    pub fn decode_base64(input: &str) -> Option<Self> {
        let trimmed = input.trim_end_matches('=');
        if trimmed.is_empty() {
            return Some(Self::ZERO);
        }
        if trimmed.len() > MAX_ENCODED_LEN || (trimmed.len() & 0x3) == 1 {
            return None;
        }

        let mut processed = [0_u8; 16];
        let mut accum: u32 = 0;
        let mut group_idx: usize = 0;
        let mut processed_idx: usize = 0;
        let mut last_decoded: u8 = 0;

        for b in trimmed.bytes() {
            let decoded = decode_b64_byte(b)?;
            last_decoded = decoded;
            accum |= u32::from(decoded);
            group_idx += 1;
            if group_idx == 4 {
                *processed.get_mut(processed_idx)? = (accum >> 16) as u8;
                *processed.get_mut(processed_idx + 1)? = ((accum >> 8) & 0xFF) as u8;
                *processed.get_mut(processed_idx + 2)? = (accum & 0xFF) as u8;
                processed_idx += 3;
                accum = 0;
                group_idx = 0;
            } else {
                accum <<= 6;
            }
        }

        if group_idx != 0 {
            accum <<= 18 - (6 * group_idx);
            *processed.get_mut(processed_idx)? = (accum >> 16) as u8;
            if group_idx == 3 {
                if (last_decoded & 0x03) != 0 {
                    return None;
                }
                *processed.get_mut(processed_idx + 1)? = ((accum >> 8) & 0xFF) as u8;
            } else if (last_decoded & 0x0F) != 0 {
                return None;
            }
        }

        let mut lo = 0_u64;
        let mut hi = 0_u64;
        for i in (0..8).rev() {
            lo = (lo << 8) | u64::from(processed[i]);
            hi = (hi << 8) | u64::from(processed[i + 8]);
        }
        Some(Self::from_parts(lo, hi))
    }
}

impl Ord for R128 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl PartialOrd for R128 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for R128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode_base64())
    }
}

fn push_b64_char(out: &mut String, idx: u32) {
    if let Some(&ch) = BASE64_ALPHABET.get(idx as usize) {
        out.push(char::from(ch));
    }
}

const fn decode_b64_byte(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MAX_BASE64: &str = "/////////////////////w";

    #[test]
    fn serialize_zero() {
        assert!(R128::from_parts(0, 0).to_string().is_empty());
    }

    #[test]
    fn serialize_maximum() {
        assert_eq!(R128::from_parts(u64::MAX, u64::MAX).to_string(), MAX_BASE64);
    }

    #[test]
    fn deserialize_empty() {
        let val = R128::decode_base64("").expect("empty decodes to zero");
        assert_eq!(val.lo(), 0);
        assert_eq!(val.hi(), 0);
    }

    #[test]
    fn deserialize_maximum() {
        let val = R128::decode_base64(MAX_BASE64).expect("max decodes");
        assert_eq!(val.lo(), u64::MAX);
        assert_eq!(val.hi(), u64::MAX);
    }

    #[test]
    fn ignore_trailing_equals() {
        let mut value = String::from("test/+01");
        let expected = R128::decode_base64(&value).expect("valid base64");
        for _ in 1..10 {
            value.push('=');
            let actual = R128::decode_base64(&value).expect("trailing '=' ignored");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn invalid_character_rejected() {
        assert_eq!(R128::decode_base64("foo$bar"), None);
    }

    #[test]
    fn reject_overlong() {
        assert_eq!(R128::decode_base64("antidisestablishmentarianism"), None);
    }

    #[test]
    fn check_final_character() {
        for length in 1..MAX_BASE64.len() {
            let mut bytes = vec![b'/'; length];
            for (i, &ch) in BASE64_ALPHABET.iter().enumerate() {
                bytes[length - 1] = ch;
                let expected = match length % 4 {
                    0 => true,
                    1 => false,
                    2 => (i & 0xF) == 0,
                    3 => (i & 0x3) == 0,
                    _ => unreachable!(),
                };
                let s = std::str::from_utf8(&bytes).unwrap();
                assert_eq!(
                    R128::decode_base64(s).is_some(),
                    expected,
                    "length={length}, idx={i}"
                );
            }
        }
    }

    #[test]
    fn round_trip_multi_byte_base64() {
        // Deterministic PRNG covering 2..=16 byte widths matching r128_test.cpp
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next_u64 = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        let mut lo_mask: u64 = 0xFF;
        let mut hi_mask: u64 = 0;
        for _n_bytes in 2..=16 {
            hi_mask = (hi_mask << 8) | (lo_mask >> 56);
            lo_mask = (lo_mask << 8) | 0xFF;
            for _ in 0..500 {
                let value = R128::from_parts(next_u64() & lo_mask, next_u64() & hi_mask);
                let encoded = value.encode_base64();
                let decoded = R128::decode_base64(&encoded);
                assert_eq!(decoded, Some(value));
            }
        }
    }

    #[test]
    fn arithmetic_and_bounds() {
        let half = R128::from_f64(0.5);
        let one = half.checked_add(half).unwrap();
        assert!((one.to_f64() - 1.0).abs() < 1e-15);

        let two = R128::from_f64(2.0);
        let neg_one = R128::from_f64(-1.0);
        assert_eq!(two.checked_mul(neg_one), Some(R128::from_f64(-2.0)));
        assert_eq!(R128::MAX.checked_add(R128::from_raw(1)), None);
        assert_eq!(R128::MIN.checked_neg(), None);

        assert!(!R128::ZERO.is_out_of_bounds());
        assert!(R128::from_parts(0, (1_u64 << 62) + 1).is_out_of_bounds());
    }

    proptest! {
        #[test]
        fn r128_add_sub_roundtrip(a in -1_000_000.0_f64..1_000_000.0, b in -1_000_000.0_f64..1_000_000.0) {
            let ra = R128::from_f64(a);
            let rb = R128::from_f64(b);
            let sum = ra.checked_add(rb).unwrap();
            let diff = sum.checked_sub(rb).unwrap();
            prop_assert_eq!(diff, ra);
        }

        #[test]
        fn r128_base64_roundtrip_arbitrary(lo in any::<u64>(), hi in any::<u64>()) {
            let value = R128::from_parts(lo, hi);
            let encoded = value.encode_base64();
            prop_assert_eq!(R128::decode_base64(&encoded), Some(value));
        }
    }
}
