//! Fundamental astronomical constants matching Celestia `celastro/astro.h` & `celastro/date.h`.

use alestia_math::deg_to_rad;

/// Angle between J2000.0 mean equator and the ecliptic plane (`23 deg 26' 21".448`) in radians.
pub const J2000_OBLIQUITY: f64 = deg_to_rad(23.439_291_1);

/// Speed of light in `km/s` (CODATA 2022).
pub const SPEED_OF_LIGHT: f64 = 299_792.458;

/// Newtonian constant of gravitation in `N m^2 / kg^2` (CODATA 2022).
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;

/// Solar mass in kg (IAU 2015 Resolution B3 + CODATA 2022).
pub const SOLAR_MASS: f64 = 1.327_124_4e20 / GRAVITATIONAL_CONSTANT;

/// Earth mass in kg (IAU 2015 Resolution B3 + CODATA 2022).
pub const EARTH_MASS: f64 = 3.986_004e14 / GRAVITATIONAL_CONSTANT;

/// Lunar mass in kg.
pub const LUNAR_MASS: f64 = 7.346e22;

/// Jupiter mass in kg (IAU 2015 Resolution B3 + CODATA 2022).
pub const JUPITER_MASS: f64 = 1.266_865_3e17 / GRAVITATIONAL_CONSTANT;

/// Solar visual absolute magnitude (Johnson V filter).
pub const SOLAR_ABSMAG: f32 = 4.81;

/// Solar effective temperature in Kelvin.
pub const SOLAR_TEMPERATURE: f32 = 5772.0;

/// Solar bolometric magnitude (IAU 2015 Resolution B2).
pub const SOLAR_BOLOMETRIC_MAG: f32 = 4.74;

/// Solar total irradiance at 1 AU in `W / m^2` (IAU 2015 Resolution B3).
pub const SOLAR_IRRADIANCE: f32 = 1361.0;

/// Solar luminosity in Watts.
pub const SOLAR_POWER: f32 = 3.828e26;

/// Vega irradiance for Johnson V filter in `W / m^2` (Bessel 1979).
pub const VEGAN_IRRADIANCE: f32 = 3.640e-11;

/// Pogson magnitude scale base factor (`5 / ln(100)`).
pub const LN_MAG: f32 = 1.085_736_2;

/// Lowest linear screen irradiance threshold (`1 / 255`).
pub const LOWEST_IRRADIATION: f32 = 1.0 / 255.0;

/// Lowest sRGB-encoded screen irradiance step (`1 / (255 * 12.92)`).
pub const LOWEST_IRRADIATION_SRGB: f32 = 1.0 / (255.0 * 12.92);

/// Distance corresponding to 10 parsecs in light-years.
pub const LY_PER_10PARSEC: f64 = 32.615_637_771_674_336;

/// Light-years per parsec.
pub const LY_PER_PARSEC: f64 = 3.261_563_777_167_433_6;

/// Kilometers per Julian light-year.
pub const KM_PER_LY: f64 = 9_460_730_472_580.8;

/// Kilometers per Astronomical Unit (IAU 2012 exact).
pub const KM_PER_AU: f64 = 149_597_870.7;

/// Astronomical Units per Julian light-year.
pub const AU_PER_LY: f64 = 63_241.077_084_266_28;

/// Kilometers per parsec.
pub const KM_PER_PARSEC: f64 = 3.085_677_581_491_367_3e13;

/// Arcminutes per degree.
pub const MINUTES_PER_DEG: f64 = 60.0;

/// Arcseconds per degree.
pub const SECONDS_PER_DEG: f64 = 3600.0;

/// Degrees per hour of right ascension.
pub const DEG_PER_HRA: f64 = 15.0;

/// Earth equatorial radius in km (IAU 2015 Resolution B3).
pub const EARTH_RADIUS: f64 = 6378.1;

/// Jupiter equatorial radius in km (IAU 2015 Resolution B3).
pub const JUPITER_RADIUS: f64 = 71492.0;

/// Solar nominal radius in km (IAU 2015 Resolution B3).
pub const SOLAR_RADIUS: f64 = 695_700.0;

/// J2000.0 epoch (`2000-01-01T12:00:00`) as Julian Date.
pub const J2000: f64 = 2_451_545.0;

/// Days per Julian year.
pub const DAYS_PER_YEAR: f64 = 365.25;

/// Seconds per day.
pub const SECONDS_PER_DAY: f64 = 86400.0;

/// Minutes per day.
pub const MINUTES_PER_DAY: f64 = 1440.0;

/// Hours per day.
pub const HOURS_PER_DAY: f64 = 24.0;

/// Constant offset (`TT - TAI`) in seconds.
pub const DTA_SECONDS: f64 = 32.184;
