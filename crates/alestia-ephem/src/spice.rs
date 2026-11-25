//! Safe optional NASA/NAIF `SPICE` ephemeris & attitude abstraction gated behind the `spice`
//! crate feature (`FR-012` / `FR-014`).
//!
//! Designed with `#![deny(unsafe_code)]` so Rust and WASM targets can configure `SPICE` kernel
//! manifests, `SPK` orbits (`SpiceOrbit`), and `CK`/`FK`/`PCK` orientations (`SpiceRotation`)
//! through a clean safe trait backend [`SpiceStateProvider`] without mandatory native C FFI.

use core::fmt;
use std::sync::Arc;

use alestia_astro::J2000_OBLIQUITY;
use alestia_math::{DMat3, DQuat, DVec3, x_rotation};

use crate::orbit::{Orbit, OrbitSample, adaptive_sample_orbit};
use crate::rotation::RotationModel;

const DEFAULT_SOLAR_SYSTEM_BOUNDING_KM: f64 = 1.0e9;

/// Supported NASA/NAIF `SPICE` kernel file categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpiceKernelKind {
    /// Spacecraft and planetary ephemeris kernel (`.bsp`).
    Spk,
    /// Planetary constants kernel (`.tpc` / `.bpc`).
    Pck,
    /// Leap-seconds kernel (`.tls`).
    Lsk,
    /// Reference frame kernel (`.tf`).
    Fk,
    /// C-matrix attitude kernel (`.bc`).
    Ck,
    /// Spacecraft clock kernel (`.tsc`).
    Sclk,
}

impl SpiceKernelKind {
    /// Infer kernel kind from a filename's extension (case-insensitive).
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        let lower = ext.to_ascii_lowercase();
        match lower.as_str() {
            "bsp" => Some(Self::Spk),
            "tpc" | "bpc" => Some(Self::Pck),
            "tls" => Some(Self::Lsk),
            "tf" => Some(Self::Fk),
            "bc" => Some(Self::Ck),
            "tsc" => Some(Self::Sclk),
            _ => None,
        }
    }
}

/// Registered `SPICE` kernel entry declared in an ephemeris or rotation definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpiceKernelEntry {
    /// Logical path or asset identifier of the kernel.
    pub name: String,
    /// Kernel category inferred from extension or explicitly assigned.
    pub kind: SpiceKernelKind,
}

/// Ordered collection of `SPICE` kernels required by a trajectory or rotation model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpiceKernelManifest {
    entries: Vec<SpiceKernelEntry>,
}

impl SpiceKernelManifest {
    /// Create an empty kernel manifest.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Append a kernel filename, validating its extension.
    ///
    /// # Errors
    /// Returns [`SpiceError::UnsupportedKernel`] if the filename extension is not recognized.
    pub fn push_named(&mut self, name: impl Into<String>) -> Result<(), SpiceError> {
        let name = name.into();
        let ext = name.rsplit('.').next().unwrap_or("");
        let kind = SpiceKernelKind::from_extension(ext)
            .ok_or_else(|| SpiceError::UnsupportedKernel(name.clone()))?;
        self.entries.push(SpiceKernelEntry { name, kind });
        Ok(())
    }

    /// Slice of registered kernels in load order.
    #[must_use]
    pub fn entries(&self) -> &[SpiceKernelEntry] {
        &self.entries
    }
}

/// Errors raised by `SPICE` manifest setup or state evaluation backends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpiceError {
    /// Kernel filename has an unsupported or missing extension.
    UnsupportedKernel(String),
    /// Requested target, origin, or frame identifier could not be resolved.
    UnknownIdentifier(String),
    /// Requested epoch fell outside kernel coverage.
    InsufficientCoverage {
        /// Epoch queried in TDB Julian Date.
        tdb_days: i64,
    },
    /// Arbitrary backend diagnostic message.
    Backend(String),
}

impl fmt::Display for SpiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedKernel(name) => {
                write!(f, "unsupported SPICE kernel extension for '{name}'")
            }
            Self::UnknownIdentifier(name) => write!(f, "unknown SPICE body or frame '{name}'"),
            Self::InsufficientCoverage { tdb_days } => {
                write!(f, "SPICE kernel lacks coverage near TDB JD {tdb_days}")
            }
            Self::Backend(msg) => write!(f, "SPICE backend error: {msg}"),
        }
    }
}

impl std::error::Error for SpiceError {}

/// Position and velocity state in J2000 equatorial kilometers and `km/day`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpiceState {
    /// J2000 equatorial position vector in kilometers.
    pub position_km: DVec3,
    /// J2000 equatorial velocity vector in kilometers per day.
    pub velocity_km_per_day: DVec3,
}

/// Safe backend contract implemented by `CSPICE`, `ANISE`, or synthetic kernel evaluators.
pub trait SpiceStateProvider: Send + Sync {
    /// Evaluate target J2000 equatorial state relative to `origin` in `frame` at TDB Julian Date.
    ///
    /// # Errors
    /// Returns [`SpiceError`] if coverage is missing or body identifiers are unknown.
    fn compute_state(
        &self,
        target: &str,
        origin: &str,
        frame: &str,
        tdb: f64,
    ) -> Result<SpiceState, SpiceError>;

    /// Compute the `3x3` rotation matrix transforming from `from_frame` to `to_frame` at `tdb`.
    ///
    /// # Errors
    /// Returns [`SpiceError`] if either frame cannot be resolved at `tdb`.
    fn frame_transform(
        &self,
        from_frame: &str,
        to_frame: &str,
        tdb: f64,
    ) -> Result<DMat3, SpiceError>;
}

/// Ephemeris orbit backed by a safe [`SpiceStateProvider`] (`celephem/spiceorbit.h`).
pub struct SpiceOrbit {
    provider: Arc<dyn SpiceStateProvider>,
    manifest: SpiceKernelManifest,
    target: String,
    origin: String,
    frame: String,
    period: f64,
    bounding_radius: f64,
    valid_range: (f64, f64),
}

impl SpiceOrbit {
    /// Construct a `SPICE`-backed orbit evaluating `target` relative to `origin` in `frame`
    /// (typically `"J2000"` or `"ECLIPJ2000"`).
    #[must_use]
    pub fn new(
        provider: Arc<dyn SpiceStateProvider>,
        manifest: SpiceKernelManifest,
        target: impl Into<String>,
        origin: impl Into<String>,
        frame: impl Into<String>,
        period: f64,
        bounding_radius: f64,
    ) -> Self {
        Self {
            provider,
            manifest,
            target: target.into(),
            origin: origin.into(),
            frame: frame.into(),
            period: period.max(0.0),
            bounding_radius: if bounding_radius > 0.0 {
                bounding_radius
            } else {
                DEFAULT_SOLAR_SYSTEM_BOUNDING_KM
            },
            valid_range: (0.0, 0.0),
        }
    }

    /// Attach explicit valid TDB interval `[begin, end]`.
    #[must_use]
    pub const fn with_valid_range(mut self, begin: f64, end: f64) -> Self {
        self.valid_range = (begin, end);
        self
    }

    /// Declared kernel manifest required by this orbit.
    #[must_use]
    pub const fn manifest(&self) -> &SpiceKernelManifest {
        &self.manifest
    }

    fn j2000_to_celestia_ecliptic(vec: DVec3) -> DVec3 {
        let ecl = x_rotation(-J2000_OBLIQUITY) * vec;
        DVec3::new(ecl.x, ecl.z, -ecl.y)
    }
}

impl Orbit for SpiceOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        self.provider
            .compute_state(&self.target, &self.origin, &self.frame, tdb)
            .map_or(DVec3::ZERO, |s| {
                Self::j2000_to_celestia_ecliptic(s.position_km)
            })
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        self.provider
            .compute_state(&self.target, &self.origin, &self.frame, tdb)
            .map_or(DVec3::ZERO, |s| {
                Self::j2000_to_celestia_ecliptic(s.velocity_km_per_day)
            })
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn is_periodic(&self) -> bool {
        self.period > 0.0
    }

    fn valid_range(&self) -> (f64, f64) {
        self.valid_range
    }

    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        adaptive_sample_orbit(self, start_time, end_time)
    }
}

/// Attitude rotation model backed by a safe [`SpiceStateProvider`] (`celephem/spicerotation.h`).
pub struct SpiceRotation {
    provider: Arc<dyn SpiceStateProvider>,
    manifest: SpiceKernelManifest,
    frame: String,
    base_frame: String,
    period: f64,
    valid_range: (f64, f64),
}

impl SpiceRotation {
    /// Construct a `SPICE`-backed rotation transforming from `frame` relative to `base_frame`.
    #[must_use]
    pub fn new(
        provider: Arc<dyn SpiceStateProvider>,
        manifest: SpiceKernelManifest,
        frame: impl Into<String>,
        base_frame: impl Into<String>,
        period: f64,
    ) -> Self {
        Self {
            provider,
            manifest,
            frame: frame.into(),
            base_frame: base_frame.into(),
            period: period.max(0.0),
            valid_range: (0.0, 0.0),
        }
    }

    /// Attach explicit valid TDB interval `[begin, end]`.
    #[must_use]
    pub const fn with_valid_range(mut self, begin: f64, end: f64) -> Self {
        self.valid_range = (begin, end);
        self
    }

    /// Declared kernel manifest required by this rotation model.
    #[must_use]
    pub const fn manifest(&self) -> &SpiceKernelManifest {
        &self.manifest
    }
}

impl RotationModel for SpiceRotation {
    fn spin(&self, tdb: f64) -> DQuat {
        self.provider
            .frame_transform(&self.frame, &self.base_frame, tdb)
            .map_or(DQuat::IDENTITY, |mat| DQuat::from_mat3(&mat))
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn is_periodic(&self) -> bool {
        self.period > 0.0
    }

    fn valid_range(&self) -> (f64, f64) {
        self.valid_range
    }
}

#[cfg(test)]
mod tests {
    use core::f64::consts::TAU;

    use alestia_astro::{J2000, KM_PER_AU};

    use super::*;

    struct MockSpiceProvider;

    impl SpiceStateProvider for MockSpiceProvider {
        fn compute_state(
            &self,
            target: &str,
            _origin: &str,
            _frame: &str,
            tdb: f64,
        ) -> Result<SpiceState, SpiceError> {
            if target != "EARTH" {
                return Err(SpiceError::UnknownIdentifier(target.to_owned()));
            }
            let theta = TAU * ((tdb - J2000) / 365.25);
            let r = KM_PER_AU;
            let omega = TAU / 365.25;
            Ok(SpiceState {
                position_km: DVec3::new(r * theta.cos(), r * theta.sin(), 0.0),
                velocity_km_per_day: DVec3::new(
                    -r * omega * theta.sin(),
                    r * omega * theta.cos(),
                    0.0,
                ),
            })
        }

        fn frame_transform(
            &self,
            from_frame: &str,
            _to_frame: &str,
            tdb: f64,
        ) -> Result<DMat3, SpiceError> {
            if from_frame != "IAU_EARTH" {
                return Err(SpiceError::UnknownIdentifier(from_frame.to_owned()));
            }
            Ok(DMat3::from_rotation_z(TAU * (tdb - J2000)))
        }
    }

    #[test]
    fn spice_manifest_orbit_and_rotation_evaluate_safely() -> Result<(), SpiceError> {
        let mut manifest = SpiceKernelManifest::new();
        manifest.push_named("de440.bsp")?;
        manifest.push_named("naif0012.tls")?;
        manifest.push_named("pck00011.tpc")?;
        assert_eq!(manifest.entries().len(), 3);
        assert!(matches!(
            manifest.push_named("unknown.xyz"),
            Err(SpiceError::UnsupportedKernel(_))
        ));

        let provider = Arc::new(MockSpiceProvider);
        let orbit = SpiceOrbit::new(
            Arc::clone(&provider) as Arc<dyn SpiceStateProvider>,
            manifest.clone(),
            "EARTH",
            "SSB",
            "J2000",
            365.25,
            2.0 * KM_PER_AU,
        )
        .with_valid_range(J2000 - 100.0, J2000 + 100.0);
        assert!(orbit.is_periodic());
        assert_eq!(orbit.valid_range(), (J2000 - 100.0, J2000 + 100.0));
        assert!((orbit.position_at_time(J2000).length() - KM_PER_AU).abs() < 1e-6);
        assert!(orbit.velocity_at_time(J2000).length() > 1.0e6);
        assert_eq!(orbit.manifest().entries().len(), 3);

        let rot = SpiceRotation::new(provider, manifest, "IAU_EARTH", "J2000", 1.0)
            .with_valid_range(J2000 - 10.0, J2000 + 10.0);
        assert!(rot.is_periodic());
        assert_eq!(rot.valid_range(), (J2000 - 10.0, J2000 + 10.0));
        let q = rot.orientation_at_time(J2000 + 0.25);
        assert!(q.is_normalized());
        assert_eq!(rot.manifest().entries().len(), 3);
        Ok(())
    }
}
