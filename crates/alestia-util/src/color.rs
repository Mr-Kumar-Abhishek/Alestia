//! RGBA color representation and sRGB/linear color space conversions.
//!
//! Matches Celestia's `Color` (`src/celutil/color.h`), storing 8-bit normalized channels
//! alongside IEC 61966-2-1 sRGB <-> linear-light conversion helpers.

/// 4-channel RGBA color matching Celestia's `Color` representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    rgba: [u8; 4],
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Color {
    /// Opaque black (`#000000ff`).
    pub const BLACK: Self = Self::from_rgba8(0, 0, 0, 255);

    /// Opaque white (`#ffffffff`).
    pub const WHITE: Self = Self::from_rgba8(255, 255, 255, 255);

    /// Construct from 8-bit RGBA components.
    #[must_use]
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    /// Construct from 8-bit RGB components with alpha `255`.
    #[must_use]
    pub const fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba8(r, g, b, 255)
    }

    /// Construct from normalized `f32` RGBA components in `[0.0, 1.0]`.
    #[must_use]
    pub fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            rgba: [
                quantize_channel(r),
                quantize_channel(g),
                quantize_channel(b),
                quantize_channel(a),
            ],
        }
    }

    /// Construct from normalized `f32` RGB components with alpha `1.0`.
    #[must_use]
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::from_rgba(r, g, b, 1.0)
    }

    /// Parse a `#rrggbb` or `#rrggbbaa` hexadecimal color string.
    #[must_use]
    pub fn parse_hex(input: &str) -> Option<Self> {
        let hex = input.strip_prefix('#')?;
        match hex.len() {
            6 => {
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::from_rgb8(red, green, blue))
            }
            8 => {
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let alpha = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::from_rgba8(red, green, blue, alpha))
            }
            _ => None,
        }
    }

    /// Red channel normalized to `[0.0, 1.0]`.
    #[must_use]
    pub fn red(self) -> f32 {
        f32::from(self.rgba[0]) / 255.0
    }

    /// Green channel normalized to `[0.0, 1.0]`.
    #[must_use]
    pub fn green(self) -> f32 {
        f32::from(self.rgba[1]) / 255.0
    }

    /// Blue channel normalized to `[0.0, 1.0]`.
    #[must_use]
    pub fn blue(self) -> f32 {
        f32::from(self.rgba[2]) / 255.0
    }

    /// Alpha channel normalized to `[0.0, 1.0]`.
    #[must_use]
    pub fn alpha(self) -> f32 {
        f32::from(self.rgba[3]) / 255.0
    }

    /// Raw 8-bit `[r, g, b, a]` channels.
    #[must_use]
    pub const fn to_rgba8(self) -> [u8; 4] {
        self.rgba
    }

    /// Convert sRGB color channels to linear-light `[r, g, b, a]` (alpha stays linear).
    #[must_use]
    pub fn to_linear_rgba(self) -> [f32; 4] {
        [
            srgb_to_linear(self.red()),
            srgb_to_linear(self.green()),
            srgb_to_linear(self.blue()),
            self.alpha(),
        ]
    }

    /// Convert linear-light `[r, g, b, a]` into an sRGB [`Color`].
    #[must_use]
    pub fn from_linear_rgba(rgba: [f32; 4]) -> Self {
        Self::from_rgba(
            linear_to_srgb(rgba[0]),
            linear_to_srgb(rgba[1]),
            linear_to_srgb(rgba[2]),
            rgba[3],
        )
    }
}

/// Convert a single sRGB channel `[0.0, 1.0]` to linear light (IEC 61966-2-1).
#[must_use]
pub fn srgb_to_linear(c: f32) -> f32 {
    let clamped = c.clamp(0.0, 1.0);
    if clamped <= 0.04045 {
        clamped / 12.92
    } else {
        ((clamped + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert a single linear-light channel `[0.0, 1.0]` to sRGB (IEC 61966-2-1).
#[must_use]
pub fn linear_to_srgb(c: f32) -> f32 {
    let clamped = c.clamp(0.0, 1.0);
    if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize_channel(v: f32) -> u8 {
    if !v.is_finite() {
        return 0;
    }
    (v.clamp(0.0, 1.0) * 255.99) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_rgb_and_rgba() {
        let rgb = Color::parse_hex("#123456").expect("valid hex6");
        assert_eq!(rgb.to_rgba8(), [0x12, 0x34, 0x56, 0xFF]);

        let rgba = Color::parse_hex("#12345678").expect("valid hex8");
        assert_eq!(rgba.to_rgba8(), [0x12, 0x34, 0x56, 0x78]);

        assert_eq!(Color::parse_hex("123456"), None);
        assert_eq!(Color::parse_hex("#12345"), None);
        assert_eq!(Color::parse_hex("#zzzzzz"), None);
    }

    #[test]
    fn srgb_linear_roundtrip_accurate() {
        for i in 0..=100 {
            #[allow(clippy::cast_precision_loss)]
            let srgb = (i as f32) / 100.0;
            let lin = srgb_to_linear(srgb);
            let back = linear_to_srgb(lin);
            assert!((back - srgb).abs() < 1e-6, "srgb={srgb}, back={back}");
        }
    }
}
