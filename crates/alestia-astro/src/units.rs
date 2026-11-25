//! Unit scale tables (`celastro/units.h`), astronomical distance conversions, and sexagesimal formatters.

use core::f64::consts::PI;

use crate::constants::{
    AU_PER_LY, DAYS_PER_YEAR, DEG_PER_HRA, EARTH_MASS, EARTH_RADIUS, HOURS_PER_DAY, JUPITER_MASS,
    JUPITER_RADIUS, KM_PER_AU, KM_PER_LY, KM_PER_PARSEC, LY_PER_PARSEC, MINUTES_PER_DAY,
    MINUTES_PER_DEG, SECONDS_PER_DAY, SECONDS_PER_DEG, SOLAR_RADIUS,
};

/// Length units supported in Celestia catalog declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    /// Default unit (unspecified).
    Default = 0,
    /// Kilometers (`1.0 km`).
    Kilometer,
    /// Meters (`1e-3 km`).
    Meter,
    /// Centimeters (`1e-5 km`).
    Centimeter,
    /// Earth equatorial radii.
    EarthRadius,
    /// Jupiter equatorial radii.
    JupiterRadius,
    /// Solar radii.
    SolarRadius,
    /// Astronomical Units.
    AstronomicalUnit,
    /// Light-years.
    LightYear,
    /// Parsecs.
    Parsec,
    /// Kiloparsecs.
    Kiloparsec,
    /// Megaparsecs.
    Megaparsec,
}

/// Time units supported in Celestia catalog declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    /// Default unit (unspecified).
    Default = 0,
    /// Seconds.
    Second,
    /// Minutes.
    Minute,
    /// Hours.
    Hour,
    /// Julian days (`86400 s`).
    Day,
    /// Julian years (`365.25 days`).
    JulianYear,
}

/// Angle units supported in Celestia catalog declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AngleUnit {
    /// Default unit (unspecified).
    Default = 0,
    /// Milliarcseconds.
    Milliarcsecond,
    /// Arcseconds.
    Arcsecond,
    /// Arcminutes.
    Arcminute,
    /// Degrees.
    Degree,
    /// Hours of right ascension (`15 deg`).
    Hour,
    /// Radians.
    Radian,
}

/// Mass units supported in Celestia catalog declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MassUnit {
    /// Default unit (unspecified).
    Default = 0,
    /// Kilograms (scaled relative to Earth mass).
    Kilogram,
    /// Earth masses (`1.0`).
    EarthMass,
    /// Jupiter masses (relative to Earth mass).
    JupiterMass,
}

/// Scale factor converting `unit` into kilometers, returning `None` for `LengthUnit::Default`.
#[must_use]
pub const fn length_scale(unit: LengthUnit) -> Option<f64> {
    match unit {
        LengthUnit::Default => None,
        LengthUnit::Kilometer => Some(1.0),
        LengthUnit::Meter => Some(1e-3),
        LengthUnit::Centimeter => Some(1e-5),
        LengthUnit::EarthRadius => Some(EARTH_RADIUS),
        LengthUnit::JupiterRadius => Some(JUPITER_RADIUS),
        LengthUnit::SolarRadius => Some(SOLAR_RADIUS),
        LengthUnit::AstronomicalUnit => Some(KM_PER_AU),
        LengthUnit::LightYear => Some(KM_PER_LY),
        LengthUnit::Parsec => Some(KM_PER_PARSEC),
        LengthUnit::Kiloparsec => Some(1e3 * KM_PER_PARSEC),
        LengthUnit::Megaparsec => Some(1e6 * KM_PER_PARSEC),
    }
}

/// Scale factor converting `unit` into Julian days, returning `None` for `TimeUnit::Default`.
#[must_use]
pub const fn time_scale(unit: TimeUnit) -> Option<f64> {
    match unit {
        TimeUnit::Default => None,
        TimeUnit::Second => Some(1.0 / SECONDS_PER_DAY),
        TimeUnit::Minute => Some(1.0 / MINUTES_PER_DAY),
        TimeUnit::Hour => Some(1.0 / HOURS_PER_DAY),
        TimeUnit::Day => Some(1.0),
        TimeUnit::JulianYear => Some(DAYS_PER_YEAR),
    }
}

/// Scale factor converting `unit` into degrees, returning `None` for `AngleUnit::Default`.
#[must_use]
pub const fn angle_scale(unit: AngleUnit) -> Option<f64> {
    match unit {
        AngleUnit::Default => None,
        AngleUnit::Milliarcsecond => Some(1e-3 / SECONDS_PER_DEG),
        AngleUnit::Arcsecond => Some(1.0 / SECONDS_PER_DEG),
        AngleUnit::Arcminute => Some(1.0 / MINUTES_PER_DEG),
        AngleUnit::Degree => Some(1.0),
        AngleUnit::Hour => Some(DEG_PER_HRA),
        AngleUnit::Radian => Some(180.0 / PI),
    }
}

/// Scale factor converting `unit` into Earth masses, returning `None` for `MassUnit::Default`.
#[must_use]
pub const fn mass_scale(unit: MassUnit) -> Option<f64> {
    match unit {
        MassUnit::Default => None,
        MassUnit::Kilogram => Some(1.0 / EARTH_MASS),
        MassUnit::EarthMass => Some(1.0),
        MassUnit::JupiterMass => Some(JUPITER_MASS / EARTH_MASS),
    }
}

/// Convert light-years to parsecs.
#[must_use]
pub const fn light_years_to_parsecs(ly: f64) -> f64 {
    ly / LY_PER_PARSEC
}

/// Convert parsecs to light-years.
#[must_use]
pub const fn parsecs_to_light_years(pc: f64) -> f64 {
    pc * LY_PER_PARSEC
}

/// Convert light-years to kilometers (`f64`).
#[must_use]
pub const fn light_years_to_kilometers(ly: f64) -> f64 {
    ly * KM_PER_LY
}

/// Convert light-years to kilometers (`f32`).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Matches C++ lightYearsToKilometers<float> overload"
)]
pub const fn light_years_to_kilometers_f32(ly: f32) -> f32 {
    ly * (KM_PER_LY as f32)
}

/// Convert kilometers to light-years.
#[must_use]
pub const fn kilometers_to_light_years(km: f64) -> f64 {
    km / KM_PER_LY
}

/// Convert light-years to Astronomical Units.
#[must_use]
pub const fn light_years_to_au(ly: f64) -> f64 {
    ly * AU_PER_LY
}

/// Convert Astronomical Units to light-years.
#[must_use]
pub const fn au_to_light_years(au: f64) -> f64 {
    au / AU_PER_LY
}

/// Convert Astronomical Units to kilometers.
#[must_use]
pub const fn au_to_kilometers(au: f64) -> f64 {
    au * KM_PER_AU
}

/// Convert kilometers to Astronomical Units.
#[must_use]
pub const fn kilometers_to_au(km: f64) -> f64 {
    km / KM_PER_AU
}

/// Convert micro-light-years to kilometers.
#[must_use]
pub const fn micro_light_years_to_kilometers(mly: f64) -> f64 {
    mly * (KM_PER_LY * 1e-6)
}

/// Convert kilometers to micro-light-years.
#[must_use]
pub const fn kilometers_to_micro_light_years(km: f64) -> f64 {
    km / (KM_PER_LY * 1e-6)
}

/// Convert micro-light-years to Astronomical Units.
#[must_use]
pub const fn micro_light_years_to_au(mly: f64) -> f64 {
    mly * (AU_PER_LY * 1e-6)
}

/// Convert Astronomical Units to micro-light-years.
#[must_use]
pub const fn au_to_micro_light_years(au: f64) -> f64 {
    au / (AU_PER_LY * 1e-6)
}

/// Convert a decimal angle in degrees into `(degrees, minutes, seconds)`.
#[must_use]
pub fn decimal_to_deg_min_sec(angle_deg: f64) -> (i32, i32, f64) {
    let whole_deg = trunc_to_i32(angle_deg);
    let rem_min = (angle_deg - f64::from(whole_deg)) * 60.0;
    let whole_min = trunc_to_i32(rem_min);
    let secs = (rem_min - f64::from(whole_min)) * 60.0;
    (whole_deg, whole_min, secs)
}

/// Convert `(degrees, minutes, seconds)` into decimal degrees.
#[must_use]
pub fn deg_min_sec_to_decimal(degrees: i32, minutes: i32, seconds: f64) -> f64 {
    f64::from(degrees) + (seconds / 60.0 + f64::from(minutes)) / 60.0
}

/// Convert a decimal angle in degrees (`0..360`) into `(hours, minutes, seconds)` of right ascension.
#[must_use]
pub fn decimal_to_hour_min_sec(angle_deg: f64) -> (i32, i32, f64) {
    let hours_float = angle_deg / 15.0;
    let whole_hours = trunc_to_i32(hours_float);
    let rem_min = (hours_float - f64::from(whole_hours)) * 60.0;
    let whole_min = trunc_to_i32(rem_min);
    let secs = (rem_min - f64::from(whole_min)) * 60.0;
    (whole_hours, whole_min, secs)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Angle sexagesimal integer component explicitly clamped to i32 range"
)]
fn trunc_to_i32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value
        .trunc()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::{
        AngleUnit, LengthUnit, MassUnit, TimeUnit, angle_scale, au_to_kilometers,
        au_to_light_years, au_to_micro_light_years, decimal_to_deg_min_sec,
        decimal_to_hour_min_sec, deg_min_sec_to_decimal, kilometers_to_au,
        kilometers_to_light_years, kilometers_to_micro_light_years, length_scale,
        light_years_to_au, light_years_to_kilometers, light_years_to_kilometers_f32,
        light_years_to_parsecs, mass_scale, micro_light_years_to_au,
        micro_light_years_to_kilometers, parsecs_to_light_years, time_scale,
    };

    #[test]
    fn unit_scales_and_distance_roundtrips_match_iau_constants() -> Result<(), String> {
        assert_eq!(length_scale(LengthUnit::Default), None);
        assert_eq!(time_scale(TimeUnit::Default), None);
        assert_eq!(angle_scale(AngleUnit::Default), None);
        assert_eq!(mass_scale(MassUnit::Default), None);

        if length_scale(LengthUnit::AstronomicalUnit) != Some(149_597_870.7) {
            return Err("AU length scale mismatch".to_owned());
        }
        if time_scale(TimeUnit::JulianYear) != Some(365.25) {
            return Err("JulianYear time scale mismatch".to_owned());
        }
        if angle_scale(AngleUnit::Hour) != Some(15.0) {
            return Err("Hour angle scale mismatch".to_owned());
        }
        if mass_scale(MassUnit::EarthMass) != Some(1.0) {
            return Err("EarthMass scale mismatch".to_owned());
        }

        let km = light_years_to_kilometers(2.5);
        if (kilometers_to_light_years(km) - 2.5).abs() > 1e-13 {
            return Err("LY<->km roundtrip drifted".to_owned());
        }
        if (light_years_to_kilometers_f32(1.0) - 9.460_73e12_f32).abs() > 1e8 {
            return Err("LY<->km f32 mismatch".to_owned());
        }
        let pc = light_years_to_parsecs(10.0);
        if (parsecs_to_light_years(pc) - 10.0).abs() > 1e-13 {
            return Err("LY<->pc roundtrip drifted".to_owned());
        }
        let au = light_years_to_au(1.0);
        if (au_to_light_years(au) - 1.0).abs() > 1e-14 {
            return Err("LY<->AU roundtrip drifted".to_owned());
        }
        if (kilometers_to_au(au_to_kilometers(1.0)) - 1.0).abs() > 1e-14 {
            return Err("AU<->km roundtrip drifted".to_owned());
        }
        let mly = kilometers_to_micro_light_years(micro_light_years_to_kilometers(42.0));
        if (mly - 42.0).abs() > 1e-12 {
            return Err("mly<->km roundtrip drifted".to_owned());
        }
        let mly_au = au_to_micro_light_years(micro_light_years_to_au(42.0));
        if (mly_au - 42.0).abs() > 1e-12 {
            return Err("mly<->AU roundtrip drifted".to_owned());
        }
        Ok(())
    }

    #[test]
    fn sexagesimal_conversions_roundtrip() -> Result<(), String> {
        let dec = deg_min_sec_to_decimal(23, 26, 21.448);
        let (deg, min, sec) = decimal_to_deg_min_sec(dec);
        if deg != 23 || min != 26 || (sec - 21.448).abs() > 1e-9 {
            return Err(format!("DMS mismatch: ({deg}, {min}, {sec})"));
        }
        let (hr, hmin, hsec) = decimal_to_hour_min_sec(187.5);
        if hr != 12 || hmin != 30 || hsec.abs() > 1e-9 {
            return Err(format!("HMS mismatch: ({hr}, {hmin}, {hsec})"));
        }
        Ok(())
    }
}
