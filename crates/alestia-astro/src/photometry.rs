//! Photometric luminosity, visual magnitude, and Vega-normalized irradiance conversions
//! matching `celastro/astro.h` and `celastro/astro.cpp` (`FR-009`).

use crate::constants::{KM_PER_PARSEC, LN_MAG, LOWEST_IRRADIATION, LY_PER_PARSEC, SOLAR_ABSMAG};

/// Luminosity of a perfectly reflective disc (used as an upper irradiance bound for culling).
///
/// `distance_from_sun` and `obj_radius` must be in the same length unit.
#[must_use]
pub fn reflected_luminosity(sun_luminosity: f32, distance_from_sun: f32, obj_radius: f32) -> f32 {
    let ratio = obj_radius / distance_from_sun;
    sun_luminosity * 0.25 * ratio * ratio
}

/// Distance modulus `5 * log10(distance_ly / LY_PER_PARSEC) - 5`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Matches Celestia distanceModulus float32 return type"
)]
pub fn distance_modulus(lyrs: f64) -> f32 {
    (5.0 * (lyrs / LY_PER_PARSEC).log10() - 5.0) as f32
}

/// Convert absolute visual magnitude and distance in light-years to apparent visual magnitude.
#[must_use]
pub fn abs_to_app_mag(abs_mag: f32, lyrs: f64) -> f32 {
    abs_mag + distance_modulus(lyrs)
}

/// Convert apparent visual magnitude and distance in light-years to absolute visual magnitude.
#[must_use]
pub fn app_to_abs_mag(app_mag: f32, lyrs: f64) -> f32 {
    app_mag - distance_modulus(lyrs)
}

/// Convert solar-relative visual luminosity to absolute visual magnitude.
#[must_use]
pub fn lum_to_abs_mag(lum: f32) -> f32 {
    SOLAR_ABSMAG - lum.ln() * LN_MAG
}

/// Convert solar-relative visual luminosity and distance in light-years to apparent magnitude.
#[must_use]
pub fn lum_to_app_mag(lum: f32, lyrs: f64) -> f32 {
    abs_to_app_mag(lum_to_abs_mag(lum), lyrs)
}

/// Convert apparent visual magnitude into Vega-normalized linear irradiance (`mag 0 => 1.0`).
#[must_use]
pub fn mag_to_irradiance(mag: f32) -> f32 {
    (-mag / LN_MAG).exp()
}

/// Convert Vega-normalized linear irradiance back into apparent visual magnitude.
#[must_use]
pub fn irradiance_to_mag(irradiance: f32) -> f32 {
    -irradiance.ln() * LN_MAG
}

/// Convert solar-relative visual luminosity and distance in km into Vega-normalized irradiance.
///
/// Retains `f64` normalization at 10 parsecs to prevent overflow on extreme luminosities/distances.
#[must_use]
pub fn lum_to_irradiance(lum: f32, km: f32) -> f32 {
    lum_to_irradiance_impl(f64::from(lum), km)
}

/// Convert absolute visual magnitude to solar-relative visual luminosity.
#[must_use]
pub fn abs_mag_to_lum(mag: f32) -> f32 {
    ((SOLAR_ABSMAG - mag) / LN_MAG).exp()
}

/// Convert apparent visual magnitude and distance in light-years to solar-relative luminosity.
#[must_use]
pub fn app_mag_to_lum(mag: f32, lyrs: f64) -> f32 {
    abs_mag_to_lum(app_to_abs_mag(mag, lyrs))
}

/// Convert absolute visual magnitude and distance in km directly into Vega-normalized irradiance,
/// avoiding `f32` overflow/underflow of intermediate solar luminosity.
#[must_use]
pub fn abs_mag_to_irradiance(mag: f32, km: f32) -> f32 {
    let luminosity = ((f64::from(SOLAR_ABSMAG) - f64::from(mag)) / f64::from(LN_MAG)).exp();
    lum_to_irradiance_impl(luminosity, km)
}

/// Convert faintest visible star magnitude threshold into renderer exposure scale.
#[must_use]
pub fn faintest_mag_to_exposure(faintest_mag: f32) -> f32 {
    (faintest_mag / LN_MAG).exp() * LOWEST_IRRADIATION
}

/// Convert renderer exposure scale into faintest visible star magnitude threshold.
#[must_use]
pub fn exposure_to_faintest_mag(exposure: f32) -> f32 {
    (exposure / LOWEST_IRRADIATION).ln() * LN_MAG
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Normalizes in f64 before casting to f32 matching Celestia lumToIrradianceImpl"
)]
fn lum_to_irradiance_impl(luminosity: f64, km: f32) -> f32 {
    let solar_irradiance_at_10pc = f64::from(mag_to_irradiance(SOLAR_ABSMAG));
    let distance_ratio = (10.0 * KM_PER_PARSEC) / f64::from(km);
    (luminosity * solar_irradiance_at_10pc * distance_ratio * distance_ratio) as f32
}

#[cfg(test)]
mod tests {
    use super::{
        abs_mag_to_irradiance, abs_mag_to_lum, abs_to_app_mag, app_mag_to_lum, app_to_abs_mag,
        exposure_to_faintest_mag, faintest_mag_to_exposure, irradiance_to_mag, lum_to_abs_mag,
        lum_to_app_mag, lum_to_irradiance, mag_to_irradiance, reflected_luminosity,
    };
    use crate::constants::LY_PER_10PARSEC;
    use crate::units::light_years_to_kilometers_f32;

    #[test]
    fn distant_irradiance_follows_inverse_square_falloff() -> Result<(), String> {
        let distance = light_years_to_kilometers_f32(1.0);
        let reference = lum_to_irradiance(1.0, distance);
        for scale in [1000.0_f32, 10_000.0_f32] {
            let actual = lum_to_irradiance(1.0, distance * scale);
            if actual <= 0.0 {
                return Err(format!("Expected positive irradiance for scale {scale}"));
            }
            let product = actual * scale * scale;
            if ((product - reference) / reference).abs() > 1e-5 {
                return Err(format!("Inverse square drift: {product} vs {reference}"));
            }
        }
        Ok(())
    }

    #[test]
    fn extreme_finite_source_and_distance_retain_representable_irradiance() -> Result<(), String> {
        let distance = light_years_to_kilometers_f32(1.0);
        let reference = f64::from(lum_to_irradiance(1.0, distance));
        for (lum, dist) in [(1.0e20_f32, distance), (f32::MAX, f32::MAX)] {
            let actual = lum_to_irradiance(lum, dist);
            if !actual.is_finite() || actual <= 0.0 {
                return Err(format!("Non-finite/non-positive irradiance: {actual}"));
            }
            let dist_ratio = f64::from(dist) / f64::from(distance);
            let scaled = f64::from(actual) / f64::from(lum) * dist_ratio * dist_ratio;
            if ((scaled - reference) / reference).abs() > 1e-5 {
                return Err(format!("Extreme irradiance scaled={scaled} vs {reference}"));
            }
        }
        Ok(())
    }

    #[test]
    fn abs_mag_to_irradiance_matches_beyond_f32_luminosity_range() -> Result<(), String> {
        for (mag, dist_ly) in [(-100.0_f32, 10_000.0_f32), (120.0_f32, 0.000_001_f32)] {
            let expected = mag_to_irradiance(abs_to_app_mag(mag, f64::from(dist_ly)));
            let actual = abs_mag_to_irradiance(mag, light_years_to_kilometers_f32(dist_ly));
            if !expected.is_finite() || !actual.is_finite() || expected <= 0.0 {
                return Err(format!(
                    "Invalid irradiance values: expected={expected}, actual={actual}"
                ));
            }
            if ((actual / expected) - 1.0).abs() > 1e-5 {
                return Err(format!(
                    "abs_mag_to_irradiance ratio mismatch: {actual}/{expected}"
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn magnitude_luminosity_and_exposure_roundtrips_hold() -> Result<(), String> {
        if (abs_to_app_mag(4.81, LY_PER_10PARSEC) - 4.81).abs() > 1e-5 {
            return Err("At 10 parsecs abs and app mag must match".to_owned());
        }
        if (app_to_abs_mag(0.0, LY_PER_10PARSEC)).abs() > 1e-5 {
            return Err("app_to_abs_mag at 10pc mismatch".to_owned());
        }
        if (lum_to_abs_mag(abs_mag_to_lum(4.81)) - 4.81).abs() > 1e-5 {
            return Err("lum<->absmag roundtrip failed".to_owned());
        }
        if (app_mag_to_lum(lum_to_app_mag(2.0, 100.0), 100.0) - 2.0).abs() > 1e-4 {
            return Err("app_mag_to_lum roundtrip failed".to_owned());
        }
        if (irradiance_to_mag(mag_to_irradiance(2.5)) - 2.5).abs() > 1e-5 {
            return Err("mag<->irradiance roundtrip failed".to_owned());
        }
        if (exposure_to_faintest_mag(faintest_mag_to_exposure(6.5)) - 6.5).abs() > 1e-4 {
            return Err("exposure<->faintest_mag roundtrip failed".to_owned());
        }
        if (reflected_luminosity(1.0, 10.0, 2.0) - 0.01).abs() > 1e-7 {
            return Err("reflected_luminosity formula mismatch".to_owned());
        }
        Ok(())
    }
}
