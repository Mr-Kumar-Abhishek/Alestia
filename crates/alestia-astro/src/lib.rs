#![forbid(unsafe_code)]
//! Astronomical timekeeping (`JD`/`UTC`/`TAI`/`TT`/`TDB`), NIST leap seconds,
//! unit scales, coordinate frames, and photometry conversions for Alestia.
//!
//! # Examples
//!
//! ```
//! use alestia_astro::{Date, J2000, abs_mag_to_irradiance, light_years_to_kilometers_f32};
//!
//! let parsed = Date::parse("2000-01-01T12:00:00Z");
//! assert!(parsed.is_some_and(|d| (d.to_julian_date() - J2000).abs() < 1e-12));
//!
//! let irr = abs_mag_to_irradiance(4.81, light_years_to_kilometers_f32(32.615638));
//! assert!(irr > 0.0);
//! ```

pub mod constants;
pub mod coords;
pub mod date;
pub mod photometry;
pub mod units;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] =
    &[alestia_math::CRATE_VERSION, alestia_util::CRATE_VERSION];

pub use constants::{
    AU_PER_LY, DAYS_PER_YEAR, DEG_PER_HRA, DTA_SECONDS, EARTH_MASS, EARTH_RADIUS,
    GRAVITATIONAL_CONSTANT, HOURS_PER_DAY, J2000, J2000_OBLIQUITY, JUPITER_MASS, JUPITER_RADIUS,
    KM_PER_AU, KM_PER_LY, KM_PER_PARSEC, LN_MAG, LOWEST_IRRADIATION, LOWEST_IRRADIATION_SRGB,
    LUNAR_MASS, LY_PER_10PARSEC, LY_PER_PARSEC, MINUTES_PER_DAY, MINUTES_PER_DEG, SECONDS_PER_DAY,
    SECONDS_PER_DEG, SOLAR_ABSMAG, SOLAR_BOLOMETRIC_MAG, SOLAR_IRRADIANCE, SOLAR_MASS, SOLAR_POWER,
    SOLAR_RADIUS, SOLAR_TEMPERATURE, SPEED_OF_LIGHT, VEGAN_IRRADIANCE,
};
pub use coords::{
    KeplerElements, anomaly, ecliptic_to_equatorial, ecliptic_to_equatorial_quat,
    equatorial_to_celestial_cart, equatorial_to_celestial_cart_f32, equatorial_to_galactic,
    equatorial_to_galactic_quat, mean_ecliptic_obliquity, state_vector_to_elements,
};
pub use date::{
    DEFAULT_LEAP_SECONDS, Date, LeapSecondRecord, LeapSecondTable, LeapSecondsParseError,
    days_to_secs, jd_utc_to_tai, julian_date_to_seconds, seconds_to_julian_date, secs_to_days,
    tai_to_jd_utc, tai_to_tt, tai_to_utc, tdb_correction, tdb_to_tt, tdb_to_utc, tt_to_tai,
    tt_to_tdb, utc_to_tai, utc_to_tdb,
};
pub use photometry::{
    abs_mag_to_irradiance, abs_mag_to_lum, abs_to_app_mag, app_mag_to_lum, app_to_abs_mag,
    distance_modulus, exposure_to_faintest_mag, faintest_mag_to_exposure, irradiance_to_mag,
    lum_to_abs_mag, lum_to_app_mag, lum_to_irradiance, mag_to_irradiance, reflected_luminosity,
};
pub use units::{
    AngleUnit, LengthUnit, MassUnit, TimeUnit, angle_scale, au_to_kilometers, au_to_light_years,
    au_to_micro_light_years, decimal_to_deg_min_sec, decimal_to_hour_min_sec,
    deg_min_sec_to_decimal, kilometers_to_au, kilometers_to_light_years,
    kilometers_to_micro_light_years, length_scale, light_years_to_au, light_years_to_kilometers,
    light_years_to_kilometers_f32, light_years_to_parsecs, mass_scale, micro_light_years_to_au,
    micro_light_years_to_kilometers, parsecs_to_light_years, time_scale,
};
