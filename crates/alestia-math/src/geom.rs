//! 3D rays, spheres, ellipsoids, ray intersections, rotations, and projections
//! matching `celmath/ray.h`, `sphere.h`, `ellipsoid.h`, `distance.h`, `intersect.h`, and `geomutil.h`.

use core::f64::consts::FRAC_PI_2;

use glam::{DMat3, DMat4, DQuat, DVec3, DVec4, Mat3, Mat4, Quat, Vec3};

use crate::mathlib::deg_to_rad;

/// Double-precision 3D parametric ray `origin + t * direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    /// Ray origin in 3D space.
    pub origin: DVec3,
    /// Ray direction vector.
    pub direction: DVec3,
}

impl Ray3 {
    /// Construct a new ray from `origin` and `direction`.
    #[must_use]
    pub const fn new(origin: DVec3, direction: DVec3) -> Self {
        Self { origin, direction }
    }

    /// Evaluate point along the ray at parameter `t`.
    #[must_use]
    pub fn point_at(self, t: f64) -> DVec3 {
        self.origin + self.direction * t
    }

    /// Transform both origin and direction by a unit quaternion.
    #[must_use]
    pub fn transform_quat(self, rotation: DQuat) -> Self {
        Self::new(rotation * self.origin, rotation * self.direction)
    }

    /// Transform both origin and direction by a 3x3 matrix.
    #[must_use]
    pub fn transform_mat3(self, matrix: DMat3) -> Self {
        Self::new(matrix * self.origin, matrix * self.direction)
    }

    /// Transform origin (as point, `w = 1`) and direction (as vector, `w = 0`) by a 4x4 matrix.
    #[must_use]
    pub fn transform_mat4(self, matrix: DMat4) -> Self {
        Self::new(
            matrix.transform_point3(self.origin),
            matrix.transform_vector3(self.direction),
        )
    }

    /// Euclidean distance from `point` to the forward half-line (`t >= 0`).
    #[must_use]
    pub fn distance_to_point(self, point: DVec3) -> f64 {
        let dir_sq = self.direction.length_squared();
        if dir_sq == 0.0 || !dir_sq.is_finite() {
            return (point - self.origin).length();
        }
        let param_t = (point - self.origin).dot(self.direction) / dir_sq;
        if param_t <= 0.0 {
            (point - self.origin).length()
        } else {
            (point - self.point_at(param_t)).length()
        }
    }
}

/// Single-precision 3D parametric ray `origin + t * direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3f {
    /// Ray origin in 3D space.
    pub origin: Vec3,
    /// Ray direction vector.
    pub direction: Vec3,
}

impl Ray3f {
    /// Construct a new single-precision ray.
    #[must_use]
    pub const fn new(origin: Vec3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    /// Evaluate point along the ray at parameter `t`.
    #[must_use]
    pub fn point_at(self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Transform by a unit quaternion.
    #[must_use]
    pub fn transform_quat(self, rotation: Quat) -> Self {
        Self::new(rotation * self.origin, rotation * self.direction)
    }

    /// Transform by a 3x3 matrix.
    #[must_use]
    pub fn transform_mat3(self, matrix: Mat3) -> Self {
        Self::new(matrix * self.origin, matrix * self.direction)
    }

    /// Transform by a 4x4 matrix.
    #[must_use]
    pub fn transform_mat4(self, matrix: Mat4) -> Self {
        Self::new(
            matrix.transform_point3(self.origin),
            matrix.transform_vector3(self.direction),
        )
    }

    /// Euclidean distance from `point` to the forward half-line (`t >= 0`).
    #[must_use]
    pub fn distance_to_point(self, point: Vec3) -> f32 {
        let dir_sq = self.direction.length_squared();
        if dir_sq == 0.0 || !dir_sq.is_finite() {
            return (point - self.origin).length();
        }
        let param_t = (point - self.origin).dot(self.direction) / dir_sq;
        if param_t <= 0.0 {
            (point - self.origin).length()
        } else {
            (point - self.point_at(param_t)).length()
        }
    }
}

/// Double-precision 3D sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// Sphere center.
    pub center: DVec3,
    /// Sphere radius.
    pub radius: f64,
}

impl Default for Sphere {
    fn default() -> Self {
        Self::unit()
    }
}

impl Sphere {
    /// Unit sphere centered at the origin.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(DVec3::ZERO, 1.0)
    }

    /// Sphere centered at the origin with `radius`.
    #[must_use]
    pub const fn from_radius(radius: f64) -> Self {
        Self::new(DVec3::ZERO, radius)
    }

    /// Sphere with `center` and `radius`.
    #[must_use]
    pub const fn new(center: DVec3, radius: f64) -> Self {
        Self { center, radius }
    }

    /// Return `true` when `point` lies inside or on the sphere boundary.
    #[must_use]
    pub fn contains(self, point: DVec3) -> bool {
        self.radius >= 0.0 && (point - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Single-precision 3D sphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spheref {
    /// Sphere center.
    pub center: Vec3,
    /// Sphere radius.
    pub radius: f32,
}

impl Default for Spheref {
    fn default() -> Self {
        Self::unit()
    }
}

impl Spheref {
    /// Unit sphere centered at the origin.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(Vec3::ZERO, 1.0)
    }

    /// Sphere centered at the origin with `radius`.
    #[must_use]
    pub const fn from_radius(radius: f32) -> Self {
        Self::new(Vec3::ZERO, radius)
    }

    /// Sphere with `center` and `radius`.
    #[must_use]
    pub const fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Return `true` when `point` lies inside or on the sphere boundary.
    #[must_use]
    pub fn contains(self, point: Vec3) -> bool {
        self.radius >= 0.0 && (point - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Double-precision axis-aligned 3D ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoid {
    /// Ellipsoid center.
    pub center: DVec3,
    /// Positive semi-axis lengths `(rx, ry, rz)`.
    pub axes: DVec3,
}

impl Default for Ellipsoid {
    fn default() -> Self {
        Self::unit()
    }
}

impl Ellipsoid {
    /// Unit ellipsoid (unit sphere) centered at the origin.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(DVec3::ZERO, DVec3::ONE)
    }

    /// Ellipsoid centered at the origin with `axes`.
    #[must_use]
    pub const fn from_axes(axes: DVec3) -> Self {
        Self::new(DVec3::ZERO, axes)
    }

    /// Ellipsoid with `center` and semi-axes `axes`.
    #[must_use]
    pub const fn new(center: DVec3, axes: DVec3) -> Self {
        Self { center, axes }
    }

    /// Return `true` when all semi-axes are strictly positive and `point` lies inside or on the ellipsoid.
    #[must_use]
    pub fn contains(self, point: DVec3) -> bool {
        if self.axes.x <= 0.0 || self.axes.y <= 0.0 || self.axes.z <= 0.0 {
            return false;
        }
        let scaled = (point - self.center) / self.axes;
        scaled.length_squared() <= 1.0
    }
}

/// Single-precision axis-aligned 3D ellipsoid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ellipsoidf {
    /// Ellipsoid center.
    pub center: Vec3,
    /// Positive semi-axis lengths `(rx, ry, rz)`.
    pub axes: Vec3,
}

impl Default for Ellipsoidf {
    fn default() -> Self {
        Self::unit()
    }
}

impl Ellipsoidf {
    /// Unit ellipsoid centered at the origin.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(Vec3::ZERO, Vec3::ONE)
    }

    /// Ellipsoid centered at the origin with `axes`.
    #[must_use]
    pub const fn from_axes(axes: Vec3) -> Self {
        Self::new(Vec3::ZERO, axes)
    }

    /// Ellipsoid with `center` and semi-axes `axes`.
    #[must_use]
    pub const fn new(center: Vec3, axes: Vec3) -> Self {
        Self { center, axes }
    }

    /// Return `true` when all semi-axes are strictly positive and `point` lies inside or on the ellipsoid.
    #[must_use]
    pub fn contains(self, point: Vec3) -> bool {
        if self.axes.x <= 0.0 || self.axes.y <= 0.0 || self.axes.z <= 0.0 {
            return false;
        }
        let scaled = (point - self.center) / self.axes;
        scaled.length_squared() <= 1.0
    }
}

/// Result of a forward ray intersection query including center-angle cosine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// Nearest positive ray parameter `t` at intersection (`point_at(distance)`).
    pub distance: f64,
    /// Cosine of angle between ray direction and vector from ray origin to primitive center.
    pub cos_angle_to_center: f64,
}

fn smallest_positive_quadratic_root(quad_a: f64, quad_b: f64, quad_c: f64) -> Option<f64> {
    if !quad_a.is_finite() || !quad_b.is_finite() || !quad_c.is_finite() || quad_a == 0.0 {
        return None;
    }
    let disc = quad_b * quad_b - quad_a * quad_c;
    if disc < 0.0 || !disc.is_finite() {
        return None;
    }
    let root_disc = disc.sqrt();
    let sol0 = (-quad_b + root_disc) / quad_a;
    let sol1 = (-quad_b - root_disc) / quad_a;

    if sol0 > 0.0 {
        if sol0 < sol1 || sol1 <= 0.0 {
            Some(sol0)
        } else {
            Some(sol1)
        }
    } else if sol1 > 0.0 {
        Some(sol1)
    } else {
        None
    }
}

fn center_angle_cosine(ray: &Ray3, center: DVec3) -> f64 {
    let to_center = center - ray.origin;
    let norm = to_center.length();
    if norm == 0.0 || !norm.is_finite() {
        1.0
    } else {
        to_center.dot(ray.direction) / norm
    }
}

/// Intersect a forward ray (`t > 0`) with a sphere, matching `celmath/intersect.h`.
#[must_use]
pub fn ray_sphere_intersection(ray: &Ray3, sphere: &Sphere) -> Option<f64> {
    if sphere.radius <= 0.0 || !sphere.radius.is_finite() {
        return None;
    }
    let diff = ray.origin - sphere.center;
    let inv_rad_sq = 1.0 / (sphere.radius * sphere.radius);
    let quad_a = ray.direction.length_squared() * inv_rad_sq;
    let quad_b = ray.direction.dot(diff) * inv_rad_sq;
    let quad_c = diff.length_squared() * inv_rad_sq - 1.0;
    smallest_positive_quadratic_root(quad_a, quad_b, quad_c)
}

/// Intersect a forward ray with a sphere and compute `cos_angle_to_center`.
#[must_use]
pub fn ray_sphere_intersection_with_cos(ray: &Ray3, sphere: &Sphere) -> Option<RayHit> {
    let distance = ray_sphere_intersection(ray, sphere)?;
    Some(RayHit {
        distance,
        cos_angle_to_center: center_angle_cosine(ray, sphere.center),
    })
}

/// Intersect a forward ray (`t > 0`) with an axis-aligned ellipsoid, matching `celmath/intersect.h`.
#[must_use]
pub fn ray_ellipsoid_intersection(ray: &Ray3, ellipsoid: &Ellipsoid) -> Option<f64> {
    if ellipsoid.axes.x <= 0.0
        || ellipsoid.axes.y <= 0.0
        || ellipsoid.axes.z <= 0.0
        || !ellipsoid.axes.is_finite()
    {
        return None;
    }
    let diff = ray.origin - ellipsoid.center;
    let inv_axes = DVec3::ONE / ellipsoid.axes;
    let inv_axes_sq = inv_axes * inv_axes;
    let scaled_dir = ray.direction * inv_axes_sq;
    let scaled_diff = diff * inv_axes_sq;

    let quad_a = ray.direction.dot(scaled_dir);
    let quad_b = ray.direction.dot(scaled_diff);
    let quad_c = diff.dot(scaled_diff) - 1.0;
    smallest_positive_quadratic_root(quad_a, quad_b, quad_c)
}

/// Intersect a forward ray with an ellipsoid and compute `cos_angle_to_center`.
#[must_use]
pub fn ray_ellipsoid_intersection_with_cos(ray: &Ray3, ellipsoid: &Ellipsoid) -> Option<RayHit> {
    let distance = ray_ellipsoid_intersection(ray, ellipsoid)?;
    Some(RayHit {
        distance,
        cos_angle_to_center: center_angle_cosine(ray, ellipsoid.center),
    })
}

/// Construct a quaternion representing a counter-clockwise rotation around +X matching Celestia `XRotation`.
#[must_use]
pub fn x_rotation(radians: f64) -> DQuat {
    let (sin_half, cos_half) = (radians * 0.5).sin_cos();
    DQuat::from_xyzw(sin_half, 0.0, 0.0, cos_half)
}

/// Construct a quaternion representing a counter-clockwise rotation around +Y matching Celestia `YRotation`.
#[must_use]
pub fn y_rotation(radians: f64) -> DQuat {
    let (sin_half, cos_half) = (radians * 0.5).sin_cos();
    DQuat::from_xyzw(0.0, sin_half, 0.0, cos_half)
}

/// Construct a quaternion representing a counter-clockwise rotation around +Z matching Celestia `ZRotation`.
#[must_use]
pub fn z_rotation(radians: f64) -> DQuat {
    let (sin_half, cos_half) = (radians * 0.5).sin_cos();
    DQuat::from_xyzw(0.0, 0.0, sin_half, cos_half)
}

/// Determine an orientation making `-Z` point from `from` to `to` and `+Y` align with `up`.
///
/// Matches Celestia's `LookAt` (`celmath/geomutil.h`), returning `DQuat::IDENTITY` on degenerate inputs.
#[must_use]
pub fn look_at(from: DVec3, to: DVec3, up: DVec3) -> DQuat {
    let forward = to - from;
    let forward_len = forward.length();
    if forward_len == 0.0 || !forward_len.is_finite() {
        return DQuat::IDENTITY;
    }
    let dir = forward / forward_len;
    let side = dir.cross(up);
    let side_len = side.length();
    if side_len == 0.0 || !side_len.is_finite() {
        return DQuat::IDENTITY;
    }
    let right = side / side_len;
    let ortho_up = right.cross(dir);
    let basis = DMat3::from_cols(right, ortho_up, -dir);
    DQuat::from_mat3(&basis).conjugate()
}

/// Perspective projection matrix matching Celestia's `Perspective(fovy_deg, aspect, near_z, far_z)`.
#[must_use]
pub fn perspective(fovy_deg: f64, aspect: f64, near_z: f64, far_z: f64) -> DMat4 {
    if aspect == 0.0 {
        return DMat4::IDENTITY;
    }
    let delta_z = far_z - near_z;
    if delta_z == 0.0 {
        return DMat4::IDENTITY;
    }
    let angle = deg_to_rad(fovy_deg * 0.5);
    let sine = angle.sin();
    if sine == 0.0 {
        return DMat4::IDENTITY;
    }
    let cotangent = angle.cos() / sine;

    DMat4::from_cols(
        DVec4::new(cotangent / aspect, 0.0, 0.0, 0.0),
        DVec4::new(0.0, cotangent, 0.0, 0.0),
        DVec4::new(0.0, 0.0, -(far_z + near_z) / delta_z, -1.0),
        DVec4::new(0.0, 0.0, -2.0 * near_z * far_z / delta_z, 0.0),
    )
}

/// Orthographic projection matrix matching Celestia's `Ortho`.
#[must_use]
pub fn ortho(left: f64, right: f64, bottom: f64, top: f64, near_z: f64, far_z: f64) -> DMat4 {
    let width = right - left;
    let height = top - bottom;
    let depth = far_z - near_z;
    if width == 0.0 || height == 0.0 || depth == 0.0 {
        return DMat4::IDENTITY;
    }

    DMat4::from_cols(
        DVec4::new(2.0 / width, 0.0, 0.0, 0.0),
        DVec4::new(0.0, 2.0 / height, 0.0, 0.0),
        DVec4::new(0.0, 0.0, -2.0 / depth, 0.0),
        DVec4::new(
            -(right + left) / width,
            -(top + bottom) / height,
            -(far_z + near_z) / depth,
            1.0,
        ),
    )
}

/// 2D orthographic projection matrix with `near = -1.0` and `far = 1.0`.
#[must_use]
pub fn ortho_2d(left: f64, right: f64, bottom: f64, top: f64) -> DMat4 {
    ortho(left, right, bottom, top, -1.0, 1.0)
}

/// Project a world-space point into viewport pixel coordinates using `model_view_projection`.
#[must_use]
pub fn project_perspective(
    point: DVec3,
    model_view_projection: &DMat4,
    viewport: [i32; 4],
) -> Option<DVec3> {
    let clip = *model_view_projection * point.extend(1.0);
    map_clip_to_viewport(clip, viewport)
}

/// Project a world-space point through Celestia's equidistant `fisheye` dome mapping.
#[must_use]
pub fn project_fisheye(
    point: DVec3,
    model_view: &DMat4,
    projection: &DMat4,
    viewport: [i32; 4],
) -> Option<DVec3> {
    let mut eye_pos = *model_view * point.extend(1.0);
    let radial = eye_pos.x.hypot(eye_pos.y);
    if radial != 0.0 {
        let phi = radial.atan2(-eye_pos.z);
        let ratio = (phi / FRAC_PI_2) / radial;
        eye_pos.x *= ratio;
        eye_pos.y *= ratio;
    }
    let clip = *projection * eye_pos;
    map_clip_to_viewport(clip, viewport)
}

fn map_clip_to_viewport(clip: DVec4, viewport: [i32; 4]) -> Option<DVec3> {
    if clip.w == 0.0 || !clip.w.is_finite() {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    let normalized = DVec3::splat(0.5) + ndc * 0.5;
    Some(DVec3::new(
        f64::from(viewport[0]) + normalized.x * f64::from(viewport[2]),
        f64::from(viewport[1]) + normalized.y * f64::from(viewport[3]),
        normalized.z,
    ))
}

/// Compute vertical perspective field of view in radians from viewport pixel height, screen DPI,
/// and viewer distance to screen.
#[must_use]
pub fn perspective_fov(height: f64, screen_dpi: i32, distance_to_screen: i32) -> f64 {
    if screen_dpi <= 0 || distance_to_screen <= 0 {
        return 0.0;
    }
    let half_height_mm = (height / (f64::from(screen_dpi) / 25.4)) * 0.5;
    2.0 * (half_height_mm / f64::from(distance_to_screen)).atan()
}

#[cfg(test)]
mod tests {
    use core::f64::consts::{FRAC_PI_2, PI};

    use glam::{DMat3, DMat4, DQuat, DVec3, Mat3, Mat4, Quat, Vec3};
    use proptest::prelude::*;

    use super::{
        Ellipsoid, Ellipsoidf, Ray3, Ray3f, Sphere, Spheref, look_at, ortho, ortho_2d, perspective,
        perspective_fov, project_fisheye, project_perspective, ray_ellipsoid_intersection,
        ray_ellipsoid_intersection_with_cos, ray_sphere_intersection,
        ray_sphere_intersection_with_cos, x_rotation, y_rotation, z_rotation,
    };

    #[test]
    fn ray_and_primitive_containment_behave_accurately() -> Result<(), String> {
        let ray = Ray3::new(DVec3::new(0.0, 2.0, 0.0), DVec3::X);
        if (ray.distance_to_point(DVec3::new(5.0, 5.0, 0.0)) - 3.0).abs() > 1e-12 {
            return Err("Ray distance forward mismatch".to_owned());
        }
        if (ray.distance_to_point(DVec3::new(-3.0, 2.0, 0.0)) - 3.0).abs() > 1e-12 {
            return Err("Ray distance behind origin mismatch".to_owned());
        }
        let rot = ray.transform_quat(z_rotation(FRAC_PI_2));
        if (rot.direction - DVec3::Y).length() > 1e-12 {
            return Err("Ray quat transform mismatch".to_owned());
        }
        let mat_ray = ray
            .transform_mat3(DMat3::IDENTITY)
            .transform_mat4(DMat4::IDENTITY);
        assert_eq!(mat_ray, ray);

        let ray_f32 = Ray3f::new(Vec3::ZERO, Vec3::X)
            .transform_quat(Quat::IDENTITY)
            .transform_mat3(Mat3::IDENTITY)
            .transform_mat4(Mat4::IDENTITY);
        if (ray_f32.distance_to_point(Vec3::new(1.0, 2.0, 0.0)) - 2.0).abs() > 1e-6 {
            return Err("Ray3f distance mismatch".to_owned());
        }

        assert!(Sphere::unit().contains(DVec3::new(0.5, 0.5, 0.0)));
        assert!(!Sphere::from_radius(1.0).contains(DVec3::new(2.0, 0.0, 0.0)));
        assert!(Spheref::default().contains(Vec3::ZERO));
        assert!(Ellipsoid::default().contains(DVec3::ZERO));
        assert!(
            Ellipsoid::from_axes(DVec3::new(2.0, 1.0, 3.0)).contains(DVec3::new(1.5, 0.0, 0.0))
        );
        assert!(!Ellipsoid::new(DVec3::ZERO, DVec3::new(-1.0, 1.0, 1.0)).contains(DVec3::ZERO));
        assert!(Ellipsoidf::from_axes(Vec3::ONE).contains(Vec3::ZERO));
        Ok(())
    }

    #[test]
    fn ray_sphere_and_ellipsoid_intersections_find_nearest_hit() -> Result<(), String> {
        let ray = Ray3::new(DVec3::new(-5.0, 0.0, 0.0), DVec3::X);
        let sphere = Sphere::from_radius(2.0);
        let Some(hit) = ray_sphere_intersection_with_cos(&ray, &sphere) else {
            return Err("Expected sphere hit".to_owned());
        };
        if (hit.distance - 3.0).abs() > 1e-12 || (hit.cos_angle_to_center - 1.0).abs() > 1e-12 {
            return Err(format!("Unexpected sphere hit: {hit:?}"));
        }

        let missed_ray = Ray3::new(DVec3::new(5.0, 0.0, 0.0), DVec3::X);
        assert_eq!(ray_sphere_intersection(&missed_ray, &sphere), None);

        let ellipsoid = Ellipsoid::from_axes(DVec3::new(3.0, 2.0, 1.0));
        let Some(ell_hit) = ray_ellipsoid_intersection_with_cos(&ray, &ellipsoid) else {
            return Err("Expected ellipsoid hit".to_owned());
        };
        if (ell_hit.distance - 2.0).abs() > 1e-12 {
            return Err(format!("Unexpected ellipsoid hit: {ell_hit:?}"));
        }
        assert_eq!(ray_ellipsoid_intersection(&missed_ray, &ellipsoid), None);
        Ok(())
    }

    #[test]
    fn rotations_look_at_and_projections_match_expectations() -> Result<(), String> {
        let qx = x_rotation(PI * 0.5);
        let qy = y_rotation(PI * 0.5);
        let qz = z_rotation(PI * 0.5);
        if ((qx * DVec3::Y) - DVec3::Z).length() > 1e-12
            || ((qy * DVec3::Z) - DVec3::X).length() > 1e-12
            || ((qz * DVec3::X) - DVec3::Y).length() > 1e-12
        {
            return Err("Axis rotation sign mismatch".to_owned());
        }

        let orient = look_at(DVec3::ZERO, DVec3::new(0.0, 0.0, -5.0), DVec3::Y);
        if (orient * DQuat::IDENTITY.conjugate() - DQuat::IDENTITY).length() > 1e-12 {
            return Err(format!(
                "Expected identity look_at along -Z, got {orient:?}"
            ));
        }

        let proj = perspective(60.0, 16.0 / 9.0, 0.1, 100.0);
        let viewport = [0, 0, 800, 600];
        let Some(screen) = project_perspective(DVec3::new(0.0, 0.0, -10.0), &proj, viewport) else {
            return Err("Expected centered projection".to_owned());
        };
        if (screen.x - 400.0).abs() > 1e-9 || (screen.y - 300.0).abs() > 1e-9 {
            return Err(format!("Unexpected center pixel: {screen:?}"));
        }

        let ortho_mat = ortho(-10.0, 10.0, -10.0, 10.0, 1.0, 50.0);
        let Some(fish) = project_fisheye(
            DVec3::new(1.0, 0.0, -1.0),
            &DMat4::IDENTITY,
            &ortho_mat,
            viewport,
        ) else {
            return Err("Expected fisheye projection".to_owned());
        };
        if !fish.is_finite() {
            return Err("Fisheye projection produced non-finite value".to_owned());
        }
        let _ = ortho_2d(0.0, 800.0, 0.0, 600.0);
        if perspective_fov(600.0, 96, 500) <= 0.0 {
            return Err("perspective_fov must be positive".to_owned());
        }
        Ok(())
    }

    proptest! {
        #[test]
        fn ray_sphere_hit_point_lies_on_sphere(
            radius in 0.5_f64..100.0_f64,
            offset in 1.5_f64..200.0_f64,
        ) {
            let sphere = Sphere::from_radius(radius);
            let ray = Ray3::new(DVec3::new(0.0, 0.0, radius + offset), DVec3::NEG_Z);
            let hit_dist = ray_sphere_intersection(&ray, &sphere)
                .ok_or_else(|| TestCaseError::fail("Expected hit"))?;
            let hit_pt = ray.point_at(hit_dist);
            prop_assert!((hit_pt.length() - radius).abs() < 1e-9);
        }
    }
}
