//! Deterministic pseudo-random number generator matching `celmath/randutils.h`.

/// Deterministic `SplitMix64` pseudo-random number generator for procedural starfields and testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

impl Default for Rng {
    fn default() -> Self {
        Self::from_seed(0x9E37_79B9_7F4A_7C15)
    }
}

impl Rng {
    /// Create a generator seeded from an explicit `u64` seed.
    #[must_use]
    pub const fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance the state and return a 64-bit pseudo-random unsigned integer.
    pub const fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// Generate a uniform `f64` in `[0.0, 1.0)` with 53-bit significand precision.
    pub fn next_f64(&mut self) -> f64 {
        let bits = self.next_u64();
        let hi_bits = u32::try_from(bits >> 37).unwrap_or(0);
        let lo_bits = u32::try_from((bits >> 11) & 0x03FF_FFFF).unwrap_or(0);
        (f64::from(hi_bits) * 67_108_864.0 + f64::from(lo_bits)) / 9_007_199_254_740_992.0
    }

    /// Generate a uniform `f32` in `[0.0, 1.0)` with 24-bit significand precision.
    pub fn next_f32(&mut self) -> f32 {
        let bits = self.next_u64();
        let hi_bits = u16::try_from(bits >> 48).unwrap_or(0);
        let lo_bits = u8::try_from((bits >> 40) & 0xFF).unwrap_or(0);
        (f32::from(hi_bits) * 256.0 + f32::from(lo_bits)) / 16_777_216.0
    }

    /// Generate a uniform `f64` in `[-1.0, 1.0)`.
    pub fn next_signed_f64(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn rng_is_deterministic_and_stays_in_unit_bounds() -> Result<(), String> {
        let mut first = Rng::from_seed(42);
        let mut second = Rng::from_seed(42);
        for _ in 0..128 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
        assert_ne!(Rng::default().next_u64(), first.next_u64());

        for _ in 0..256 {
            let val_f64 = first.next_f64();
            let val_f32 = first.next_f32();
            let signed = first.next_signed_f64();
            if !(0.0..1.0).contains(&val_f64) {
                return Err(format!("next_f64 out of bounds: {val_f64}"));
            }
            if !(0.0_f32..1.0_f32).contains(&val_f32) {
                return Err(format!("next_f32 out of bounds: {val_f32}"));
            }
            if !(-1.0..1.0).contains(&signed) {
                return Err(format!("next_signed_f64 out of bounds: {signed}"));
            }
        }
        Ok(())
    }
}
