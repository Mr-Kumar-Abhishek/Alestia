//! Astronomical coordinate frame transformations, obliquity, Kepler anomaly,
//! and osculating element recovery matching `celastro/astro.h` and `celastro/astro.cpp`.

use core::f64::consts::PI;

use alestia_math::{
    DMat3, DQuat, DVec3, Mat3, Vec3, cube, deg_to_rad, square, x_rotation, z_rotation,
};

use crate::constants::{J2000, J2000_OBLIQUITY};

const GALACTIC_NODE_DEG: f64 = 282.85958;
const GALACTIC_INCLINATION_DEG: f64 = 90.0 - 27.128_336_1;
const GALACTIC_LONGITUDE_AT_NODE_DEG: f64 = 32.932;

/// Osculating Keplerian orbital elements recovered from state vectors (`r`, `v`, `mu`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeplerElements {
    /// Semimajor axis (km).
    pub semimajor_axis: f64,
    /// Orbital eccentricity.
    pub eccentricity: f64,
    /// Orbital inclination (radians).
    pub inclination: f64,
    /// Longitude of ascending node (radians).
    pub long_ascending_node: f64,
    /// Argument of pericenter (radians).
    pub arg_pericenter: f64,
    /// Mean anomaly at epoch (radians).
    pub mean_anomaly: f64,
    /// Orbital period (seconds or time units consistent with `mu`).
    pub period: f64,
}

fn equatorial_to_ecliptic_matrix() -> DMat3 {
    DMat3::from_quat(x_rotation(-J2000_OBLIQUITY))
}

fn galactic_rotation_quat() -> DQuat {
    z_rotation(deg_to_rad(GALACTIC_NODE_DEG))
        * x_rotation(deg_to_rad(GALACTIC_INCLINATION_DEG))
        * z_rotation(deg_to_rad(-GALACTIC_LONGITUDE_AT_NODE_DEG))
}

/// Convert equatorial coordinates (`ra` in hours, `dec` in degrees, `distance`) into Cartesian
/// celestial (J2000 ecliptic) coordinates (`f64`).
#[must_use]
pub fn equatorial_to_celestial_cart(ra_hours: f64, dec_deg: f64, distance: f64) -> DVec3 {
    let theta = (ra_hours / 24.0) * PI * 2.0 + PI;
    let phi = (dec_deg / 90.0 - 1.0) * PI * 0.5;
    let (sin_theta, cos_theta) = theta.sin_cos();
    let (sin_phi, cos_phi) = phi.sin_cos();

    let eq_vec = DVec3::new(
        cos_theta * sin_phi * distance,
        cos_phi * distance,
        -sin_theta * sin_phi * distance,
    );
    equatorial_to_ecliptic_matrix() * eq_vec
}

/// Single-precision version of [`equatorial_to_celestial_cart`].
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Matches C++ equatorialToCelestialCart<float> overload"
)]
pub fn equatorial_to_celestial_cart_f32(ra_hours: f32, dec_deg: f32, distance: f32) -> Vec3 {
    let theta = (f64::from(ra_hours) / 24.0) * PI * 2.0 + PI;
    let phi = (f64::from(dec_deg) / 90.0 - 1.0) * PI * 0.5;
    let (sin_theta, cos_theta) = theta.sin_cos();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dist = f64::from(distance);

    let eq_vec = Vec3::new(
        (cos_theta * sin_phi * dist) as f32,
        (cos_phi * dist) as f32,
        (-sin_theta * sin_phi * dist) as f32,
    );
    let mat_d = equatorial_to_ecliptic_matrix();
    let mat_f = Mat3::from_cols(
        mat_d.x_axis.as_vec3(),
        mat_d.y_axis.as_vec3(),
        mat_d.z_axis.as_vec3(),
    );
    mat_f * eq_vec
}

/// Quaternion transforming from J2000 ecliptic coordinates to J2000 Earth equatorial coordinates.
#[must_use]
pub fn ecliptic_to_equatorial_quat() -> DQuat {
    x_rotation(-J2000_OBLIQUITY)
}

/// Rotate a vector from J2000 ecliptic coordinates into J2000 Earth equatorial coordinates.
#[must_use]
pub fn ecliptic_to_equatorial(vec: DVec3) -> DVec3 {
    equatorial_to_ecliptic_matrix().transpose() * vec
}

/// Quaternion transforming from J2000 Earth equatorial coordinates to galactic coordinates.
#[must_use]
pub fn equatorial_to_galactic_quat() -> DQuat {
    galactic_rotation_quat()
}

/// Rotate a vector from J2000 Earth equatorial coordinates into galactic coordinates.
#[must_use]
pub fn equatorial_to_galactic(vec: DVec3) -> DVec3 {
    DMat3::from_quat(galactic_rotation_quat()).transpose() * vec
}

/// Mean obliquity of the ecliptic at Julian Date `jd` in radians.
#[must_use]
pub fn mean_ecliptic_obliquity(jd: f64) -> f64 {
    let centuries = (jd - J2000) / 36525.0;
    let delta_rad = (46.815 * centuries + 0.0006 * centuries * centuries
        - 0.00181 * centuries * centuries * centuries)
        / 3600.0;
    J2000_OBLIQUITY - delta_rad
}

/// Solve Kepler's elliptical equation for `(true_anomaly, eccentric_anomaly)` from
/// `mean_anomaly` (radians) and `eccentricity` (`0 <= e < 1`).
#[must_use]
pub fn anomaly(mean_anomaly: f64, eccentricity: f64) -> (f64, f64) {
    const TOL: f64 = 1.745e-8;
    let two_pi = 2.0 * PI;
    let wrapped = mean_anomaly % two_pi;
    let mut eccentric = wrapped;
    let mut residual = 1.0;
    let mut iterations = 20_u32;

    while residual.abs() > TOL && iterations > 0 {
        residual = eccentric - eccentricity * eccentric.sin() - mean_anomaly;
        let delta = residual / (1.0 - eccentricity * eccentric.cos());
        eccentric -= delta;
        iterations -= 1;
    }

    let true_anom = 2.0
        * (((1.0 + eccentricity) / (1.0 - eccentricity)).sqrt() * (0.5 * eccentric).tan()).atan();
    (true_anom, eccentric)
}

/// Compute osculating Keplerian elements from position `pos`, velocity `vel`, and standard
/// gravitational parameter `mu`, using Celestia's Y-up ecliptic frame convention.
#[must_use]
pub fn state_vector_to_elements(pos: DVec3, vel: DVec3, mu: f64) -> KeplerElements {
    const TOLERANCE: f64 = 1e-9;

    let angular_momentum = pos.cross(vel);
    let pos_norm = pos.length();
    let mut elements = KeplerElements::default();

    let ecc_vec = vel.cross(angular_momentum) / mu - pos / pos_norm;
    elements.eccentricity = ecc_vec.length();

    elements.inclination = (angular_momentum.y / angular_momentum.length())
        .clamp(-1.0, 1.0)
        .acos();

    let node_vec = DVec3::new(angular_momentum.z, 0.0, -angular_momentum.x);
    let node_norm = node_vec.length();

    if elements.inclination < TOLERANCE {
        if elements.eccentricity >= TOLERANCE {
            elements.arg_pericenter = (ecc_vec.x / elements.eccentricity).clamp(-1.0, 1.0).acos();
            if ecc_vec.z >= 0.0 {
                elements.arg_pericenter = -elements.arg_pericenter;
            }
        }
    } else {
        elements.long_ascending_node = (node_vec.x / node_norm).clamp(-1.0, 1.0).acos();
        if node_vec.z >= 0.0 {
            elements.long_ascending_node = -elements.long_ascending_node;
        }
        if elements.eccentricity >= TOLERANCE {
            elements.arg_pericenter = (node_vec.dot(ecc_vec) / (node_norm * elements.eccentricity))
                .clamp(-1.0, 1.0)
                .acos();
            if ecc_vec.y < 0.0 {
                elements.arg_pericenter = -elements.arg_pericenter;
            }
        }
    }

    let mut true_anom = if elements.eccentricity >= TOLERANCE {
        let angle = (ecc_vec.dot(pos) / (elements.eccentricity * pos_norm))
            .clamp(-1.0, 1.0)
            .acos();
        if pos.dot(vel) < 0.0 { -angle } else { angle }
    } else if elements.inclination < TOLERANCE {
        let angle = (pos.x / pos_norm).clamp(-1.0, 1.0).acos();
        if vel.x > 0.0 { -angle } else { angle }
    } else {
        let angle = (node_vec.dot(pos) / (node_norm * pos_norm))
            .clamp(-1.0, 1.0)
            .acos();
        if node_vec.dot(vel) > 0.0 {
            -angle
        } else {
            angle
        }
    };

    // Guard unused sign edge case cleanly
    if !true_anom.is_finite() {
        true_anom = 0.0;
    }

    let (sin_nu, cos_nu) = true_anom.sin_cos();
    let ecc_sq = square(elements.eccentricity);
    if elements.eccentricity < 1.0 {
        let eccentric = ((1.0 - ecc_sq).sqrt() * sin_nu).atan2(elements.eccentricity + cos_nu);
        elements.mean_anomaly = eccentric - elements.eccentricity * eccentric.sin();
    } else {
        let sinh_h = ((ecc_sq - 1.0).sqrt() * sin_nu) / (1.0 + elements.eccentricity * cos_nu);
        let hyp_anom = sinh_h.asinh();
        elements.mean_anomaly = elements.eccentricity * sinh_h - hyp_anom;
    }

    elements.semimajor_axis = 1.0 / (2.0 / pos_norm - vel.length_squared() / mu);
    elements.period = 2.0 * PI * (cube(elements.semimajor_axis.abs()) / mu).sqrt();
    elements
}

#[cfg(test)]
mod tests {
    use alestia_math::DVec3;

    use super::{
        anomaly, ecliptic_to_equatorial, ecliptic_to_equatorial_quat, equatorial_to_celestial_cart,
        equatorial_to_celestial_cart_f32, equatorial_to_galactic, equatorial_to_galactic_quat,
        mean_ecliptic_obliquity, state_vector_to_elements,
    };
    use crate::constants::{J2000, J2000_OBLIQUITY};

    #[test]
    fn frame_rotations_are_orthonormal() -> Result<(), String> {
        let eq_v = equatorial_to_celestial_cart(6.0, 0.0, 10.0);
        if (eq_v.length() - 10.0).abs() > 1e-12 {
            return Err("Equatorial->celestial must preserve vector norm".to_owned());
        }
        let eq_f32 = equatorial_to_celestial_cart_f32(6.0, 0.0, 10.0);
        if (eq_f32.length() - 10.0).abs() > 1e-5 {
            return Err("Equatorial->celestial f32 must preserve vector norm".to_owned());
        }

        let rotated = ecliptic_to_equatorial(DVec3::new(1.0, 2.0, 3.0));
        if (rotated.length() - DVec3::new(1.0, 2.0, 3.0).length()).abs() > 1e-12 {
            return Err("ecliptic_to_equatorial norm mismatch".to_owned());
        }
        let gal = equatorial_to_galactic(DVec3::new(1.0, 2.0, 3.0));
        if (gal.length() - DVec3::new(1.0, 2.0, 3.0).length()).abs() > 1e-12 {
            return Err("equatorial_to_galactic norm mismatch".to_owned());
        }
        if !ecliptic_to_equatorial_quat().is_normalized()
            || !equatorial_to_galactic_quat().is_normalized()
        {
            return Err("Rotation quaternions must be normalized".to_owned());
        }
        if (mean_ecliptic_obliquity(J2000) - J2000_OBLIQUITY).abs() > 1e-15 {
            return Err("J2000 obliquity mismatch".to_owned());
        }
        Ok(())
    }

    #[test]
    fn anomaly_and_state_vector_recovery_agree() -> Result<(), String> {
        let (true_anom, ecc_anom) = anomaly(0.5, 0.25);
        let recovered_mean = ecc_anom - 0.25 * ecc_anom.sin();
        if (recovered_mean - 0.5).abs() > 1e-7 || true_anom <= 0.0 {
            return Err("Kepler anomaly residual too large".to_owned());
        }

        // Circular orbit in X-Z ecliptic plane around origin with mu = 1.0, r = 4.0, v = 0.5
        let pos = DVec3::new(4.0, 0.0, 0.0);
        let vel = DVec3::new(0.0, 0.0, 0.5);
        let elems = state_vector_to_elements(pos, vel, 1.0);
        if (elems.semimajor_axis - 4.0).abs() > 1e-11 || elems.eccentricity > 1e-11 {
            return Err(format!("Unexpected circular elements: {elems:?}"));
        }
        Ok(())
    }
}
