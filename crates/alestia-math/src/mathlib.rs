//! Scalar math utilities, positive floating-point modulo, ellipsoid tangents,
//! and plane-circle intersections matching `celmath/mathlib.h`.

use core::f64::consts::PI;

use glam::DVec3;

/// Convert degrees to radians (`f64`).
#[must_use]
pub const fn deg_to_rad(degrees: f64) -> f64 {
    degrees * (PI / 180.0)
}

/// Convert radians to degrees (`f64`).
#[must_use]
pub const fn rad_to_deg(radians: f64) -> f64 {
    radians * (180.0 / PI)
}

/// Convert degrees to radians (`f32`).
#[must_use]
pub const fn deg_to_rad_f32(degrees: f32) -> f32 {
    degrees * (core::f32::consts::PI / 180.0)
}

/// Convert radians to degrees (`f32`).
#[must_use]
pub const fn rad_to_deg_f32(radians: f32) -> f32 {
    radians * (180.0 / core::f32::consts::PI)
}

/// Square a value (`x * x`).
#[must_use]
pub const fn square(value: f64) -> f64 {
    value * value
}

/// Cube a value (`x * x * x`).
#[must_use]
pub const fn cube(value: f64) -> f64 {
    value * value * value
}

/// Sign function returning `-1.0`, `0.0`, or `1.0`.
#[must_use]
pub fn sign(value: f64) -> f64 {
    if value < 0.0 {
        -1.0
    } else if value > 0.0 {
        1.0
    } else {
        0.0
    }
}

/// Linear interpolation between `start` and `end` by parameter `t`.
#[must_use]
pub const fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + t * (end - start)
}

/// Positive floating-point modulo normalized into `[0.0, period)`.
///
/// Returns `0.0` when `value` or `period` is non-finite or when `period <= 0.0`.
#[must_use]
pub fn pfmod(value: f64, period: f64) -> f64 {
    if !value.is_finite() || !period.is_finite() || period <= 0.0 {
        return 0.0;
    }
    let quotient = (value / period).abs().floor();
    let wrapped = if value < 0.0 {
        value + (quotient + 1.0) * period
    } else {
        value - quotient * period
    };
    if wrapped >= period { 0.0 } else { wrapped }
}

/// Area of a circle (`pi * r^2`).
#[must_use]
pub const fn circle_area(radius: f64) -> f64 {
    PI * radius * radius
}

/// Surface area of a sphere (`4 * pi * r^2`).
#[must_use]
pub const fn sphere_area(radius: f64) -> f64 {
    4.0 * PI * radius * radius
}

/// Compute a point on the ellipsoid horizon tangent along the plane spanned by `eye` and `target_dir`.
///
/// Matches Celestia's `ellipsoidTangent` (`celmath/mathlib.h`), algebraically eliminating `ee^2`
/// terms to preserve precision at large observer distances.
///
/// - `recip_semi_axes`: component-wise reciprocals `(1/a, 1/b, 1/c)` of ellipsoid semi-axes.
/// - `target_dir`: vector toward the tangent limb side.
/// - `eye`: eye position relative to the ellipsoid center.
/// - `eye_scaled`: `eye * recip_semi_axes`.
/// - `eye_norm_sq`: `eye_scaled.length_squared()`.
#[must_use]
pub fn ellipsoid_tangent(
    recip_semi_axes: DVec3,
    target_dir: DVec3,
    eye: DVec3,
    eye_scaled: DVec3,
    eye_norm_sq: f64,
) -> DVec3 {
    let dir_scaled = target_dir * recip_semi_axes;
    let dir_norm_sq = dir_scaled.length_squared();
    let eye_dot_dir = dir_scaled.dot(eye_scaled);

    let quad_a = 4.0
        * (square(eye_dot_dir) - eye_norm_sq * dir_norm_sq
            + eye_norm_sq
            + 2.0 * eye_dot_dir
            + dir_norm_sq);
    let quad_b = -8.0 * (eye_norm_sq + eye_dot_dir);
    let quad_c = 4.0 * eye_norm_sq;

    let disc = (quad_b * quad_b - 4.0 * quad_a * quad_c).abs();
    let param_t = (-quad_b + disc.sqrt()) / (2.0 * quad_a);

    let ray_dir = -eye * (1.0 - param_t) + target_dir * param_t;
    let ray_scaled = ray_dir * recip_semi_axes;
    let step_a = ray_scaled.length_squared();
    let step_b = 2.0 * ray_scaled.dot(eye_scaled);
    let step_t = -step_b / (2.0 * step_a);

    eye + ray_dir * step_t
}

/// Find the two intersection points between a plane through the origin and a parametric circle
/// `center + cos(t) * axis_u + sin(t) * axis_v`.
///
/// Returns `None` when the plane and circle are parallel or intersect at fewer than two points.
#[must_use]
pub fn plane_circle_intersection(
    plane_normal: DVec3,
    center: DVec3,
    axis_u: DVec3,
    axis_v: DVec3,
) -> Option<(DVec3, DVec3)> {
    if !plane_normal.is_finite()
        || !center.is_finite()
        || !axis_u.is_finite()
        || !axis_v.is_finite()
    {
        return None;
    }

    let proj_u = axis_u.dot(plane_normal);
    let proj_v = axis_v.dot(plane_normal);
    let proj_c = center.dot(plane_normal);

    let sum_sq = proj_u * proj_u + proj_v * proj_v;
    if sum_sq == 0.0 || sum_sq - proj_c * proj_c <= 0.0 {
        return None;
    }

    let radical = proj_v * (sum_sq - proj_c * proj_c).sqrt();
    let cos0 = ((-proj_u * proj_c + radical) / sum_sq).clamp(-1.0, 1.0);
    let cos1 = ((-proj_u * proj_c - radical) / sum_sq).clamp(-1.0, 1.0);
    let sin0 = (1.0 - cos0 * cos0).max(0.0).sqrt();
    let sin1 = (1.0 - cos1 * cos1).max(0.0).sqrt();

    let mut sol0 = center + axis_u * cos0 + axis_v * sin0;
    let mut sol1 = center + axis_u * cos1 + axis_v * sin1;

    if sol0.dot(plane_normal).abs() > 1.0e-8 {
        sol0 = center + axis_u * cos0 - axis_v * sin0;
    }
    if sol1.dot(plane_normal).abs() > 1.0e-8 {
        sol1 = center + axis_u * cos1 - axis_v * sin1;
    }

    Some((sol0, sol1))
}

#[cfg(test)]
mod tests {
    use core::f64::consts::PI;

    use glam::DVec3;
    use proptest::prelude::*;

    use super::{
        circle_area, cube, deg_to_rad, deg_to_rad_f32, ellipsoid_tangent, lerp, pfmod,
        plane_circle_intersection, rad_to_deg, rad_to_deg_f32, sign, sphere_area, square,
    };

    #[test]
    fn angle_and_scalar_helpers_match_identities() -> Result<(), String> {
        if (deg_to_rad(180.0) - PI).abs() > 1e-15 {
            return Err("180 deg must equal pi".to_owned());
        }
        if (rad_to_deg(PI) - 180.0).abs() > 1e-12 {
            return Err("pi rad must equal 180 deg".to_owned());
        }
        if (deg_to_rad_f32(90.0) - core::f32::consts::FRAC_PI_2).abs() > 1e-6 {
            return Err("90 f32 deg must equal pi/2".to_owned());
        }
        if (rad_to_deg_f32(core::f32::consts::PI) - 180.0).abs() > 1e-5 {
            return Err("pi f32 rad must equal 180 deg".to_owned());
        }
        assert_eq!(square(-4.0), 16.0);
        assert_eq!(cube(-3.0), -27.0);
        assert_eq!(sign(-5.0), -1.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(9.0), 1.0);
        assert_eq!(lerp(10.0, 20.0, 0.25), 12.5);
        if (circle_area(2.0) - 4.0 * PI).abs() > 1e-14 {
            return Err("circle area mismatch".to_owned());
        }
        if (sphere_area(2.0) - 16.0 * PI).abs() > 1e-13 {
            return Err("sphere area mismatch".to_owned());
        }
        Ok(())
    }

    #[test]
    fn pfmod_handles_positive_negative_and_degenerate_inputs() -> Result<(), String> {
        if (pfmod(7.5, 2.0) - 1.5).abs() > 1e-15 {
            return Err("pfmod positive failed".to_owned());
        }
        if (pfmod(-0.5, 2.0) - 1.5).abs() > 1e-15 {
            return Err("pfmod negative failed".to_owned());
        }
        assert_eq!(pfmod(-4.0, 2.0), 0.0);
        assert_eq!(pfmod(5.0, 0.0), 0.0);
        assert_eq!(pfmod(f64::NAN, 2.0), 0.0);
        Ok(())
    }

    #[test]
    fn plane_circle_intersection_finds_two_points_and_rejects_parallel() -> Result<(), String> {
        let normal = DVec3::Z;
        let center = DVec3::new(0.0, 0.0, 0.0);
        let axis_u = DVec3::X * 5.0;
        let axis_v = DVec3::Z * 5.0;
        let Some((sol0, sol1)) = plane_circle_intersection(normal, center, axis_u, axis_v) else {
            return Err("Expected two intersection points".to_owned());
        };
        if sol0.dot(normal).abs() > 1e-9 || sol1.dot(normal).abs() > 1e-9 {
            return Err("Solutions must lie on plane".to_owned());
        }
        if (sol0.length() - 5.0).abs() > 1e-9 || (sol1.length() - 5.0).abs() > 1e-9 {
            return Err("Solutions must lie on radius-5 circle".to_owned());
        }
        if plane_circle_intersection(DVec3::Z, DVec3::Z * 10.0, DVec3::X, DVec3::Y).is_some() {
            return Err("Expected parallel offset plane/circle to return None".to_owned());
        }
        Ok(())
    }

    #[test]
    fn ellipsoid_tangent_lies_on_ellipsoid_surface() -> Result<(), String> {
        let semi_axes = DVec3::new(2.0, 1.0, 2.0);
        let recip = DVec3::ONE / semi_axes;
        let eye = DVec3::new(10.0, 0.0, 0.0);
        let eye_scaled = eye * recip;
        let target_dir = DVec3::new(0.0, 1.0, 0.0);
        let tangent = ellipsoid_tangent(
            recip,
            target_dir,
            eye,
            eye_scaled,
            eye_scaled.length_squared(),
        );
        let surface_eq = (tangent * recip).length_squared();
        if (surface_eq - 1.0).abs() > 1e-10 {
            return Err(format!(
                "Tangent point not on ellipsoid surface: {surface_eq}"
            ));
        }
        Ok(())
    }

    proptest! {
        #[test]
        fn pfmod_always_stays_within_half_open_period(
            value in -1.0e6_f64..1.0e6_f64,
            period in 0.01_f64..1.0e4_f64,
        ) {
            let result = pfmod(value, period);
            prop_assert!(result >= 0.0 && result < period);
        }

        #[test]
        fn deg_rad_roundtrip(deg in -1.0e5_f64..1.0e5_f64) {
            let back = rad_to_deg(deg_to_rad(deg));
            prop_assert!((back - deg).abs() <= 1e-8 * deg.abs().max(1.0));
        }
    }
}
