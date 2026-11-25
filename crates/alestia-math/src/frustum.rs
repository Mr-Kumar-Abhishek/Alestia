//! Viewing frustums (`Frustum` and `InfiniteFrustum`) with double-precision sphere culling
//! matching `celmath/frustum.h` and `celmath/frustum.cpp`.

use glam::{DVec3, Mat3, Mat4, Vec3, Vec4};

/// Plane index within [`Frustum`] and [`InfiniteFrustum`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum FrustumPlane {
    /// Bottom clipping plane (`0`).
    Bottom = 0,
    /// Top clipping plane (`1`).
    Top = 1,
    /// Left clipping plane (`2`).
    Left = 2,
    /// Right clipping plane (`3`).
    Right = 3,
    /// Near clipping plane (`4`).
    Near = 4,
    /// Far clipping plane (`5`, present on finite [`Frustum`] only).
    Far = 5,
}

/// Classification of a point or sphere against a viewing frustum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrustumAspect {
    /// Completely outside at least one frustum plane.
    Outside = 0,
    /// Completely inside all frustum planes.
    Inside = 1,
    /// Intersects at least one frustum plane without being outside any.
    Intersect = 2,
}

/// 3D hyperplane defined by `normal.dot(point) + offset = 0` matching Eigen `Hyperplane<float, 3>`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    /// Inward-pointing plane normal vector.
    pub normal: Vec3,
    /// Signed offset term `d` in `n . p + d = 0`.
    pub offset: f32,
}

impl Plane {
    /// Construct a plane from normal and signed offset.
    #[must_use]
    pub const fn new(normal: Vec3, offset: f32) -> Self {
        Self { normal, offset }
    }

    /// Normalize both `normal` and `offset` by `normal.length()`.
    #[must_use]
    pub fn normalized(self) -> Self {
        let len = self.normal.length();
        if len == 0.0 || !len.is_finite() {
            return self;
        }
        Self {
            normal: self.normal / len,
            offset: self.offset / len,
        }
    }

    /// Single-precision signed distance (`normal.dot(point) + offset`).
    #[must_use]
    pub fn signed_distance(self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.offset
    }

    /// Double-precision signed distance promoting plane coefficients to `f64`.
    #[must_use]
    pub fn signed_distance_f64(self, point: DVec3) -> f64 {
        let n_f64 = DVec3::new(
            f64::from(self.normal.x),
            f64::from(self.normal.y),
            f64::from(self.normal.z),
        );
        n_f64.dot(point) + f64::from(self.offset)
    }
}

/// 6-plane finite viewing frustum (`Bottom`, `Top`, `Left`, `Right`, `Near`, `Far`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// Construct a finite symmetric frustum from vertical FOV (radians), aspect ratio, near, and far distances.
    #[must_use]
    pub fn from_fov(fov_rad: f32, aspect_ratio: f32, near_dist: f32, far_dist: f32) -> Self {
        let side_planes = init_fov_planes(fov_rad, aspect_ratio, near_dist);
        let far_plane = Plane::new(Vec3::new(0.0, 0.0, 1.0), far_dist);
        Self {
            planes: [
                side_planes[0],
                side_planes[1],
                side_planes[2],
                side_planes[3],
                side_planes[4],
                far_plane,
            ],
        }
    }

    /// Construct a finite frustum from explicit near-plane bounds.
    #[must_use]
    pub fn from_bounds(
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near_dist: f32,
        far_dist: f32,
    ) -> Self {
        let side_planes = init_bounds_planes(left, right, top, bottom, near_dist);
        let far_plane = Plane::new(Vec3::new(0.0, 0.0, 1.0), far_dist);
        Self {
            planes: [
                side_planes[0],
                side_planes[1],
                side_planes[2],
                side_planes[3],
                side_planes[4],
                far_plane,
            ],
        }
    }

    /// Retrieve one of the 6 frustum clipping planes.
    #[must_use]
    pub const fn plane(&self, which: FrustumPlane) -> Plane {
        self.planes[which as usize]
    }

    /// Transform all planes by an isometric 3x3 rotation/reflection matrix.
    pub fn transform_mat3(&mut self, matrix: &Mat3) {
        transform_planes_mat3(&mut self.planes, matrix);
    }

    /// Transform all planes by an arbitrary invertible 4x4 homogeneous matrix.
    pub fn transform_mat4(&mut self, matrix: &Mat4) {
        transform_planes_mat4(&mut self.planes, matrix);
    }

    /// Classify a 3D point against the frustum (`Inside` or `Outside`).
    #[must_use]
    pub fn test_point(&self, point: Vec3) -> FrustumAspect {
        do_test_point(&self.planes, point)
    }

    /// Classify a single-precision sphere against the frustum.
    #[must_use]
    pub fn test_sphere_f32(&self, center: Vec3, radius: f32) -> FrustumAspect {
        do_test_sphere_f32(&self.planes, center, radius)
    }

    /// Classify a double-precision sphere against the frustum without downcasting `center`.
    #[must_use]
    pub fn test_sphere_f64(&self, center: DVec3, radius: f64) -> FrustumAspect {
        do_test_sphere_f64(&self.planes, center, radius)
    }
}

/// 5-plane infinite viewing frustum (`Bottom`, `Top`, `Left`, `Right`, `Near`) without a far plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InfiniteFrustum {
    planes: [Plane; 5],
}

impl InfiniteFrustum {
    /// Construct an infinite symmetric frustum from vertical FOV (radians), aspect ratio, and near distance.
    #[must_use]
    pub fn from_fov(fov_rad: f32, aspect_ratio: f32, near_dist: f32) -> Self {
        Self {
            planes: init_fov_planes(fov_rad, aspect_ratio, near_dist),
        }
    }

    /// Retrieve one of the 5 planes (`FrustumPlane::Far` returns `Near`).
    #[must_use]
    pub const fn plane(&self, which: FrustumPlane) -> Plane {
        match which {
            FrustumPlane::Far => self.planes[4],
            _ => self.planes[which as usize],
        }
    }

    /// Transform all 5 planes by an isometric 3x3 matrix.
    pub fn transform_mat3(&mut self, matrix: &Mat3) {
        transform_planes_mat3(&mut self.planes, matrix);
    }

    /// Transform all 5 planes by an invertible 4x4 matrix.
    pub fn transform_mat4(&mut self, matrix: &Mat4) {
        transform_planes_mat4(&mut self.planes, matrix);
    }

    /// Classify a 3D point against the infinite frustum.
    #[must_use]
    pub fn test_point(&self, point: Vec3) -> FrustumAspect {
        do_test_point(&self.planes, point)
    }

    /// Classify a single-precision sphere against the infinite frustum.
    #[must_use]
    pub fn test_sphere_f32(&self, center: Vec3, radius: f32) -> FrustumAspect {
        do_test_sphere_f32(&self.planes, center, radius)
    }

    /// Classify a double-precision sphere against the infinite frustum.
    #[must_use]
    pub fn test_sphere_f64(&self, center: DVec3, radius: f64) -> FrustumAspect {
        do_test_sphere_f64(&self.planes, center, radius)
    }
}

fn init_fov_planes(fov_rad: f32, aspect_ratio: f32, near_dist: f32) -> [Plane; 5] {
    let half_h = (fov_rad * 0.5).tan();
    let half_w = half_h * aspect_ratio;
    init_bounds_planes(
        -half_w * near_dist,
        half_w * near_dist,
        half_h * near_dist,
        -half_h * near_dist,
        near_dist,
    )
}

fn init_bounds_planes(left: f32, right: f32, top: f32, bottom: f32, near_dist: f32) -> [Plane; 5] {
    let normals = [
        Vec3::new(0.0, near_dist, bottom),
        Vec3::new(0.0, -near_dist, -top),
        Vec3::new(near_dist, 0.0, left),
        Vec3::new(-near_dist, 0.0, -right),
    ];
    [
        Plane::new(normals[0].normalize_or_zero(), 0.0),
        Plane::new(normals[1].normalize_or_zero(), 0.0),
        Plane::new(normals[2].normalize_or_zero(), 0.0),
        Plane::new(normals[3].normalize_or_zero(), 0.0),
        Plane::new(Vec3::new(0.0, 0.0, -1.0), -near_dist),
    ]
}

fn transform_planes_mat3(planes: &mut [Plane], matrix: &Mat3) {
    for plane in planes {
        plane.normal = *matrix * plane.normal;
    }
}

fn transform_planes_mat4(planes: &mut [Plane], matrix: &Mat4) {
    if matrix.determinant().abs() < 1e-12 {
        return;
    }
    let inv_transpose = matrix.inverse().transpose();
    for plane in planes {
        let coeffs =
            inv_transpose * Vec4::new(plane.normal.x, plane.normal.y, plane.normal.z, plane.offset);
        *plane = Plane::new(Vec3::new(coeffs.x, coeffs.y, coeffs.z), coeffs.w).normalized();
    }
}

fn do_test_point(planes: &[Plane], point: Vec3) -> FrustumAspect {
    for plane in planes {
        if plane.signed_distance(point) < 0.0 {
            return FrustumAspect::Outside;
        }
    }
    FrustumAspect::Inside
}

fn do_test_sphere_f32(planes: &[Plane], center: Vec3, radius: f32) -> FrustumAspect {
    let mut is_intersecting = false;
    for plane in planes {
        let distance = plane.signed_distance(center);
        if distance < -radius {
            return FrustumAspect::Outside;
        }
        if distance <= radius {
            is_intersecting = true;
        }
    }
    if is_intersecting {
        FrustumAspect::Intersect
    } else {
        FrustumAspect::Inside
    }
}

fn do_test_sphere_f64(planes: &[Plane], center: DVec3, radius: f64) -> FrustumAspect {
    let mut is_intersecting = false;
    for plane in planes {
        let distance = plane.signed_distance_f64(center);
        if distance < -radius {
            return FrustumAspect::Outside;
        }
        if distance <= radius {
            is_intersecting = true;
        }
    }
    if is_intersecting {
        FrustumAspect::Intersect
    } else {
        FrustumAspect::Inside
    }
}

#[cfg(test)]
mod tests {
    use core::f32::consts::FRAC_PI_3;

    use glam::{DVec3, Mat3, Mat4, Vec3};

    use super::{Frustum, FrustumAspect, FrustumPlane, InfiniteFrustum, Plane};

    #[test]
    fn finite_and_infinite_frustum_classify_points_and_spheres() {
        let frustum = Frustum::from_fov(FRAC_PI_3, 1.0, 1.0, 100.0);
        let inf_frustum = InfiniteFrustum::from_fov(FRAC_PI_3, 1.0, 1.0);

        assert_eq!(
            frustum.test_point(Vec3::new(0.0, 0.0, -10.0)),
            FrustumAspect::Inside
        );
        assert_eq!(
            frustum.test_point(Vec3::new(0.0, 0.0, -0.5)),
            FrustumAspect::Outside
        );
        assert_eq!(
            frustum.test_point(Vec3::new(0.0, 0.0, -200.0)),
            FrustumAspect::Outside
        );
        assert_eq!(
            inf_frustum.test_point(Vec3::new(0.0, 0.0, -200.0)),
            FrustumAspect::Inside
        );

        assert_eq!(
            frustum.test_sphere_f32(Vec3::new(0.0, 0.0, -20.0), 1.0),
            FrustumAspect::Inside
        );
        assert_eq!(
            frustum.test_sphere_f32(Vec3::new(0.0, 0.0, -1.5), 1.0),
            FrustumAspect::Intersect
        );
        assert_eq!(
            frustum.test_sphere_f64(DVec3::new(0.0, 0.0, -1.0e9), 1.0),
            FrustumAspect::Outside
        );
        assert_eq!(
            inf_frustum.test_sphere_f64(DVec3::new(0.0, 0.0, -1.0e12), 1.0),
            FrustumAspect::Inside
        );
    }

    #[test]
    fn frustum_transforms_preserve_geometry() {
        let mut frustum = Frustum::from_bounds(-1.0, 1.0, 1.0, -1.0, 1.0, 50.0);
        assert!(frustum.plane(FrustumPlane::Near).normal.z < 0.0);

        let rot = Mat3::from_rotation_y(core::f32::consts::FRAC_PI_2);
        frustum.transform_mat3(&rot);
        assert_eq!(
            frustum.test_point(Vec3::new(-10.0, 0.0, 0.0)),
            FrustumAspect::Inside
        );

        let mut inf = InfiniteFrustum::from_fov(FRAC_PI_3, 1.0, 1.0);
        inf.transform_mat3(&rot);
        assert_eq!(
            inf.test_sphere_f32(Vec3::new(-10.0, 0.0, 0.0), 0.5),
            FrustumAspect::Inside
        );

        let translate = Mat4::from_translation(Vec3::new(0.0, 100.0, 0.0));
        inf.transform_mat4(&translate);
        assert_eq!(
            inf.test_point(Vec3::new(-10.0, 100.0, 0.0)),
            FrustumAspect::Inside
        );
        assert_eq!(
            Plane::new(Vec3::ZERO, 1.0).normalized(),
            Plane::new(Vec3::ZERO, 1.0)
        );
    }
}
