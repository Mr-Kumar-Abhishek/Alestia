#![forbid(unsafe_code)]
//! 3D computational geometry, quaternions, frustums, intersections, and root solvers for Alestia.
//!
//! # Examples
//!
//! ```
//! use alestia_math::{DVec3, Ray3, Sphere, ray_sphere_intersection, solve_brent};
//!
//! let ray = Ray3::new(DVec3::new(-5.0, 0.0, 0.0), DVec3::X);
//! let sphere = Sphere::from_radius(2.0);
//! assert_eq!(ray_sphere_intersection(&ray, &sphere), Some(3.0));
//!
//! let solved = solve_brent(|x| x * x - 2.0, 0.0, 2.0, 1e-13, 100)?;
//! assert!((solved.root - core::f64::consts::SQRT_2).abs() < 1e-12);
//! # Ok::<(), alestia_math::SolveError>(())
//! ```

pub mod frustum;
pub mod geom;
pub mod mathlib;
pub mod rand;
pub mod solve;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Direct workspace dependency crate versions.
pub const DEPENDENCY_VERSIONS: &[&str] = &[alestia_util::CRATE_VERSION];

pub use glam::{DMat3, DMat4, DQuat, DVec2, DVec3, DVec4, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

pub use frustum::{Frustum, FrustumAspect, FrustumPlane, InfiniteFrustum, Plane};
pub use geom::{
    Ellipsoid, Ellipsoidf, Ray3, Ray3f, RayHit, Sphere, Spheref, look_at, ortho, ortho_2d,
    perspective, perspective_fov, project_fisheye, project_perspective, ray_ellipsoid_intersection,
    ray_ellipsoid_intersection_with_cos, ray_sphere_intersection, ray_sphere_intersection_with_cos,
    x_rotation, y_rotation, z_rotation,
};
pub use mathlib::{
    circle_area, cube, deg_to_rad, deg_to_rad_f32, ellipsoid_tangent, lerp, pfmod,
    plane_circle_intersection, rad_to_deg, rad_to_deg_f32, sign, sphere_area, square,
};
pub use rand::Rng;
pub use solve::{
    SolveError, SolveResult, solve_bisection, solve_brent, solve_halley, solve_iteration,
    solve_iteration_fixed, solve_newton,
};
