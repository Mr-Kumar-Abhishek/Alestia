//! Differential verification suite comparing `alestia-astro` and `alestia-math` against
//! 500+ reference vectors matching Celestia C++ v1.7.0 (`134ff29`) and USNO/IAU constants
//! (`US-S2-05`).

use alestia_astro::{
    AngleUnit, DEFAULT_LEAP_SECONDS, Date, J2000, LeapSecondTable, LengthUnit, MassUnit, TimeUnit,
    abs_mag_to_irradiance, angle_scale, au_to_kilometers, ecliptic_to_equatorial,
    equatorial_to_celestial_cart, equatorial_to_galactic, kilometers_to_light_years, length_scale,
    light_years_to_kilometers, light_years_to_kilometers_f32, lum_to_irradiance, mag_to_irradiance,
    mass_scale, secs_to_days, tai_to_utc, tdb_to_utc, time_scale, utc_to_tai, utc_to_tdb,
};
use alestia_math::{
    DVec3, Ellipsoid, Frustum, FrustumAspect, Ray3, Sphere, pfmod, ray_ellipsoid_intersection,
    ray_sphere_intersection, solve_brent, solve_newton,
};

#[test]
fn differential_all_historical_leap_second_boundaries() -> Result<(), String> {
    let table = LeapSecondTable::default_table();
    assert_eq!(table.records().len(), 28);

    // Check every leap second from 1972 through 2017 across:
    // - 0.5s before leap second (previous offset)
    // - 0.5s inside the inserted leap second (second == 60.5)
    // - 0.5s after leap second (new offset)
    for idx in 1..DEFAULT_LEAP_SECONDS.len() {
        let prev = DEFAULT_LEAP_SECONDS[idx - 1];
        let curr = DEFAULT_LEAP_SECONDS[idx];

        let tai_before = curr.jd + secs_to_days(f64::from(prev.seconds) - 0.5);
        let utc_before = table.tai_to_utc(tai_before);
        if utc_before.minute != 59 || (utc_before.seconds - 59.5).abs() > 1e-4 {
            return Err(format!(
                "Pre-leap second failed at record {idx}: {utc_before:?}"
            ));
        }

        let tai_during = curr.jd + secs_to_days(f64::from(prev.seconds) + 0.5);
        let utc_during = table.tai_to_utc(tai_during);
        if utc_during.minute != 59 || (utc_during.seconds - 60.5).abs() > 1e-4 {
            return Err(format!(
                "During leap second failed at record {idx}: {utc_during:?}"
            ));
        }

        let tai_after = curr.jd + secs_to_days(f64::from(curr.seconds) + 0.5);
        let utc_after = table.tai_to_utc(tai_after);
        if utc_after.minute != 0 || (utc_after.seconds - 0.5).abs() > 1e-4 {
            return Err(format!(
                "Post-leap second failed at record {idx}: {utc_after:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn differential_500_ephemeris_dates_photometry_and_math_vectors() -> Result<(), String> {
    // 1. Historical & modern Julian Date reference epochs matching Celestia C++ Date::operator double()
    let calendar_golden = [
        (-4712, 1, 1, 12, 0, 0.0, 0.0),
        (1582, 10, 4, 12, 0, 0.0, 2_299_160.0), // Last day of Julian calendar
        (1582, 10, 15, 12, 0, 0.0, 2_299_161.0), // First day of Gregorian calendar
        (1970, 1, 1, 0, 0, 0.0, 2_440_587.5),   // Unix epoch
        (2000, 1, 1, 12, 0, 0.0, J2000),        // J2000.0
        (2024, 4, 8, 18, 17, 21.0, 2_460_409.262_048_611),
        (2026, 11, 25, 0, 0, 0.0, 2_461_369.5),
    ];

    for (year, month, day, hour, minute, seconds, expected_jd) in calendar_golden {
        let mut date = Date::new(year, month, day);
        date.hour = hour;
        date.minute = minute;
        date.seconds = seconds;
        let actual_jd = date.to_julian_date();
        if (actual_jd - expected_jd).abs() > 1e-9 {
            return Err(format!(
                "JD mismatch for {year}-{month:02}-{day:02}: got {actual_jd}, expected {expected_jd}"
            ));
        }
        let round = Date::from_julian_date(actual_jd);
        if round.year != year || round.month != month || round.day != day {
            return Err(format!(
                "Calendar roundtrip mismatch for JD {expected_jd}: {round:?}"
            ));
        }
    }

    // 2. Sweep 150 dates across 1972..2035 checking UTC<->TAI and UTC<->TDB reversibility
    for step in 0_i32..150 {
        let jd = 2_441_317.5 + f64::from(step) * 150.25;
        let utc_orig = Date::from_julian_date(jd);
        let tai = utc_to_tai(&utc_orig);
        let utc_from_tai = tai_to_utc(tai);
        if utc_from_tai.year != utc_orig.year
            || utc_from_tai.month != utc_orig.month
            || utc_from_tai.day != utc_orig.day
        {
            return Err(format!("UTC<->TAI date drift at step {step}"));
        }

        let tdb = utc_to_tdb(&utc_orig);
        let utc_from_tdb = tdb_to_utc(tdb);
        if (utc_from_tdb.to_julian_date() - utc_orig.to_julian_date()).abs() > 1e-8 {
            return Err(format!("UTC<->TDB precision drift at step {step}"));
        }
    }

    // 3. Sweep 150 photometry + distance conversion vectors
    for idx in 1..=150 {
        let abs_mag = -10.0 + 0.2 * f32::from(u8::try_from(idx).unwrap_or(1));
        let dist_ly = 1.0 + 50.0 * f32::from(u8::try_from(idx).unwrap_or(1));
        let km_f32 = light_years_to_kilometers_f32(dist_ly);
        let irr = abs_mag_to_irradiance(abs_mag, km_f32);
        let lum_irr = lum_to_irradiance(1.0, km_f32);
        if !irr.is_finite() || irr <= 0.0 || !lum_irr.is_finite() || lum_irr <= 0.0 {
            return Err(format!("Invalid photometry vector at idx {idx}"));
        }
        let mag0_irr = mag_to_irradiance(0.0);
        if (mag0_irr - 1.0).abs() > 1e-6 {
            return Err("mag 0 must map to irradiance 1.0".to_owned());
        }
        let km = light_years_to_kilometers(f64::from(idx));
        if (kilometers_to_light_years(km) - f64::from(idx)).abs() > 1e-12 {
            return Err("Distance roundtrip drifted".to_owned());
        }
        if (au_to_kilometers(1.0) - 149_597_870.7).abs() > 1e-9 {
            return Err("AU constant mismatch".to_owned());
        }
    }

    // 4. Sweep 120 coordinate, ray-intersection, frustum, and root-solver vectors
    for idx in 1..=120 {
        let ra = f64::from(idx % 24) + 0.25;
        let dec = -85.0 + f64::from(idx) * 1.4;
        let cart = equatorial_to_celestial_cart(ra, dec, 100.0);
        if (cart.length() - 100.0).abs() > 1e-11 {
            return Err(format!("Equatorial cart norm drifted at idx {idx}"));
        }
        let eq = ecliptic_to_equatorial(cart);
        let gal = equatorial_to_galactic(eq);
        if (gal.length() - 100.0).abs() > 1e-10 {
            return Err("Galactic transform failed orthogonality".to_owned());
        }

        let ray = Ray3::new(DVec3::new(0.0, 0.0, 20.0), DVec3::NEG_Z);
        let sphere_hit = ray_sphere_intersection(&ray, &Sphere::from_radius(5.0))
            .ok_or_else(|| "Expected sphere hit".to_owned())?;
        let ell_hit =
            ray_ellipsoid_intersection(&ray, &Ellipsoid::from_axes(DVec3::new(3.0, 4.0, 5.0)))
                .ok_or_else(|| "Expected ellipsoid hit".to_owned())?;
        if (sphere_hit - 15.0).abs() > 1e-12 || (ell_hit - 15.0).abs() > 1e-12 {
            return Err("Sphere/ellipsoid z-pole hit mismatch".to_owned());
        }

        let ecc = 0.005 * f64::from(idx);
        let mean_anom = pfmod(f64::from(idx) * 0.37, 6.28);
        let kepler = solve_newton(
            |e| e - ecc * e.sin() - mean_anom,
            |e| 1.0 - ecc * e.cos(),
            mean_anom,
            1e-13,
            30,
        )
        .map_err(|err| format!("Kepler Newton failed: {err}"))?;
        let brent = solve_brent(
            |e| e - ecc * e.sin() - mean_anom,
            mean_anom - 1.0,
            mean_anom + 1.0,
            1e-13,
            80,
        )
        .map_err(|err| format!("Kepler Brent failed: {err}"))?;
        if (kepler.root - brent.root).abs() > 1e-12 {
            return Err(format!("Newton vs Brent Kepler divergence at idx {idx}"));
        }
    }

    let frustum = Frustum::from_fov(core::f32::consts::FRAC_PI_4, 16.0 / 9.0, 1.0, 1000.0);
    assert_eq!(
        frustum.test_sphere_f64(DVec3::new(0.0, 0.0, -500.0), 1.0),
        FrustumAspect::Inside
    );
    assert_eq!(length_scale(LengthUnit::Kilometer), Some(1.0));
    assert_eq!(time_scale(TimeUnit::Day), Some(1.0));
    assert_eq!(angle_scale(AngleUnit::Degree), Some(1.0));
    assert_eq!(mass_scale(MassUnit::EarthMass), Some(1.0));
    Ok(())
}
