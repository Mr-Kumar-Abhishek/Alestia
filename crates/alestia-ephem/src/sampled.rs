//! Sampled ASCII (`.xyz`, `.xyzv`, `.q`) and binary (`CELXYZV\0` `.xyzvbin`) trajectories
//! and attitude keyframes matching `celephem/samporbit.cpp`, `samporient.cpp`, `sampfile.h`,
//! and `xyzvbinary.h` (`FR-013`).

use core::f64::consts::FRAC_PI_2;
use core::fmt;
use std::sync::Mutex;

use alestia_astro::SECONDS_PER_DAY;
use alestia_math::{DQuat, DVec3, lerp, x_rotation};
use alestia_util::{BinaryError, BinaryReader, BinaryWriter};

use crate::orbit::{Orbit, OrbitSample};
use crate::rotation::RotationModel;

/// Header magic identifying Celestia binary XYZV trajectories (`CELXYZV\0`).
pub const XYZV_MAGIC: &[u8; 8] = b"CELXYZV\0";

/// Byte-order marker stored in little-endian `XYZVBIN` headers.
pub const XYZV_LITTLE_ENDIAN: u16 = 1234;

/// Byte-order marker stored in big-endian `XYZVBIN` headers.
pub const XYZV_BIG_ENDIAN: u16 = 4321;

/// Required IEEE-754 significand bit count (`std::numeric_limits<double>::digits`).
pub const XYZV_DOUBLE_DIGITS: u16 = 53;

/// Interpolation scheme used between trajectory samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrajectoryInterpolation {
    /// Piecewise linear interpolation between adjacent samples.
    Linear,
    /// Piecewise cubic Hermite / Catmull-Rom interpolation.
    Cubic,
}

/// Position-only sample stored in Celestia's internal ecliptic frame (`km`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XyzSample {
    /// Sample epoch in TDB Julian Date.
    pub time: f64,
    /// Position in kilometers (Celestia internal frame).
    pub position: DVec3,
}

/// Position and velocity sample stored in Celestia's internal ecliptic frame (`km`, `km/day`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XyzvSample {
    /// Sample epoch in TDB Julian Date.
    pub time: f64,
    /// Position in kilometers (Celestia internal frame).
    pub position: DVec3,
    /// Velocity in kilometers per day (Celestia internal frame).
    pub velocity: DVec3,
}

/// Attitude keyframe stored in Celestia's internal frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientationKeyframe {
    /// Keyframe epoch in TDB Julian Date.
    pub time: f64,
    /// Unit quaternion already converted into Celestia coordinates.
    pub rotation: DQuat,
}

/// Errors produced when parsing or constructing sampled trajectories or orientations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SampleError {
    /// Input contained zero valid monotonic samples.
    EmptySamples,
    /// Binary `XYZVBIN` header contained an invalid magic, byte order, precision, or count.
    InvalidHeader(&'static str),
    /// ASCII record contained unparseable or non-finite numeric fields.
    MalformedToken {
        /// 1-based line number where the malformed record or trailing token appeared.
        line: usize,
    },
    /// Keyframe quaternion had zero or non-finite norm.
    InvalidQuaternion,
    /// Truncated binary payload.
    Binary(BinaryError),
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples => write!(f, "sampled dataset contains no valid monotonic samples"),
            Self::InvalidHeader(msg) => write!(f, "invalid XYZV binary header: {msg}"),
            Self::MalformedToken { line } => {
                write!(
                    f,
                    "malformed numeric token in sampled file near line {line}"
                )
            }
            Self::InvalidQuaternion => {
                write!(f, "sampled orientation quaternion is zero or non-finite")
            }
            Self::Binary(err) => write!(f, "binary sample read error: {err}"),
        }
    }
}

impl std::error::Error for SampleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Binary(err) => Some(err),
            _ => None,
        }
    }
}

impl From<BinaryError> for SampleError {
    fn from(err: BinaryError) -> Self {
        Self::Binary(err)
    }
}

/// Convert standard ecliptic/equatorial `(x, y, z)` coordinates to Celestia `(x, z, -y)`.
#[must_use]
pub const fn to_celestia_coords(vec: DVec3) -> DVec3 {
    DVec3::new(vec.x, vec.z, -vec.y)
}

/// Convert Celestia internal `(x, y, z)` coordinates back to standard `(x, -z, y)`.
#[must_use]
pub const fn from_celestia_coords(vec: DVec3) -> DVec3 {
    DVec3::new(vec.x, -vec.z, vec.y)
}

#[derive(Clone, Copy)]
struct HermiteSegment {
    pos0: DVec3,
    vel0: DVec3,
    pos1: DVec3,
    vel1: DVec3,
    frac: f64,
    inv_dt: f64,
}

impl HermiteSegment {
    fn eval_position(self) -> DVec3 {
        let coeff_a = 2.0 * (self.pos0 - self.pos1) + self.vel1 + self.vel0;
        let coeff_b = 3.0 * (self.pos1 - self.pos0) - 2.0 * self.vel0 - self.vel1;
        self.pos0 + self.frac * (self.vel0 + self.frac * (coeff_b + self.frac * coeff_a))
    }

    fn eval_velocity(self) -> DVec3 {
        let cubic_coeff_3 = 3.0 * (2.0 * (self.pos0 - self.pos1) + self.vel1 + self.vel0);
        let quad_coeff_2 = 2.0 * (3.0 * (self.pos1 - self.pos0) - 2.0 * self.vel0 - self.vel1);
        (self.vel0 + self.frac * (quad_coeff_2 + self.frac * cubic_coeff_3)) * self.inv_dt
    }
}

fn find_interval_index(times: &[f64], tdb: f64) -> usize {
    times.partition_point(|&sample_t| sample_t <= tdb)
}

fn collect_ascii_tokens(input: &str) -> Vec<(usize, &str)> {
    let mut tokens = Vec::new();
    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let code = raw_line.split('#').next().unwrap_or("");
        for token in code.split_whitespace() {
            tokens.push((line_no, token));
        }
    }
    tokens
}

fn parse_finite_f64(line: usize, token: &str) -> Result<f64, SampleError> {
    let value: f64 = token
        .parse()
        .map_err(|_| SampleError::MalformedToken { line })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SampleError::MalformedToken { line })
    }
}

fn compute_bounding_radius<I>(positions: I) -> f64
where
    I: IntoIterator<Item = DVec3>,
{
    positions
        .into_iter()
        .map(DVec3::length_squared)
        .fold(0.0_f64, f64::max)
        .sqrt()
}

/// Unevenly spaced position-only trajectory (`.xyz`) with linear or Catmull-Rom cubic interpolation.
#[derive(Debug)]
pub struct SampledOrbit {
    times: Vec<f64>,
    positions: Vec<DVec3>,
    interpolation: TrajectoryInterpolation,
    bounding_radius: f64,
    last_eval: Mutex<Option<(f64, DVec3)>>,
}

impl SampledOrbit {
    /// Parse an ASCII `.xyz` trajectory (`tdb x y z` with optional `#` comments).
    ///
    /// Coordinates in the file are transformed from standard `(x, y, z)` to Celestia
    /// `(x, z, -y)` coordinates, matching upstream `LoadSamplesXYZAscii`.
    ///
    /// # Errors
    /// Returns [`SampleError::MalformedToken`] for non-numeric/non-finite fields or incomplete
    /// trailing records, or [`SampleError::EmptySamples`] if no monotonic sample exists.
    pub fn from_xyz_str(
        input: &str,
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        let tokens = collect_ascii_tokens(input);
        let mut iter = tokens.chunks_exact(4);
        let mut times = Vec::new();
        let mut positions = Vec::new();
        let mut last_time = f64::NEG_INFINITY;

        for chunk in &mut iter {
            let tdb = parse_finite_f64(chunk[0].0, chunk[0].1)?;
            let pos = DVec3::new(
                parse_finite_f64(chunk[1].0, chunk[1].1)?,
                parse_finite_f64(chunk[2].0, chunk[2].1)?,
                parse_finite_f64(chunk[3].0, chunk[3].1)?,
            );
            if tdb <= last_time {
                continue;
            }
            last_time = tdb;
            times.push(tdb);
            positions.push(to_celestia_coords(pos));
        }

        if let Some(&(line, _)) = iter.remainder().first() {
            return Err(SampleError::MalformedToken { line });
        }

        Self::from_times_and_positions(times, positions, interpolation)
    }

    /// Construct a [`SampledOrbit`] directly from [`XyzSample`] records already in Celestia coords.
    ///
    /// Out-of-order or duplicate timestamps are skipped to match upstream `LoadSamples`.
    ///
    /// # Errors
    /// Returns [`SampleError::EmptySamples`] if no monotonic sample exists.
    pub fn from_samples(
        samples: &[XyzSample],
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        let mut times = Vec::with_capacity(samples.len());
        let mut positions = Vec::with_capacity(samples.len());
        let mut last_time = f64::NEG_INFINITY;

        for sample in samples {
            if !sample.time.is_finite() || !sample.position.is_finite() {
                continue;
            }
            if sample.time <= last_time {
                continue;
            }
            last_time = sample.time;
            times.push(sample.time);
            positions.push(sample.position);
        }

        Self::from_times_and_positions(times, positions, interpolation)
    }

    fn from_times_and_positions(
        times: Vec<f64>,
        positions: Vec<DVec3>,
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        if times.is_empty() || times.len() != positions.len() {
            return Err(SampleError::EmptySamples);
        }
        let bounding_radius = compute_bounding_radius(positions.iter().copied());
        Ok(Self {
            times,
            positions,
            interpolation,
            bounding_radius,
            last_eval: Mutex::new(None),
        })
    }

    /// Number of retained monotonic samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether the sample table is empty (always `false` for constructed instances).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    fn hermite_segment(&self, tdb: f64, idx: usize) -> HermiteSegment {
        let n_max = self.times.len() - 1;
        let n0 = idx.max(2) - 2;
        let n1 = idx - 1;
        let n3 = (idx + 1).min(n_max);

        let dt = self.times[idx] - self.times[n1];
        let inv_dt = 1.0 / dt;
        let frac = (tdb - self.times[n1]) * inv_dt;
        let pos0 = self.positions[n1];
        let pos1 = self.positions[idx];

        let diff10 = pos0 - self.positions[n0];
        let diff21 = pos1 - pos0;
        let diff32 = self.positions[n3] - pos1;

        let vel0 = if idx > 1 {
            (diff10 * (0.5 / (self.times[n1] - self.times[n0])) + diff21 * (0.5 * inv_dt)) * dt
        } else {
            diff21
        };
        let vel1 = if idx < n_max {
            (diff21 * (0.5 * inv_dt) + diff32 * (0.5 / (self.times[n3] - self.times[idx]))) * dt
        } else {
            diff21
        };

        HermiteSegment {
            pos0,
            vel0,
            pos1,
            vel1,
            frac,
            inv_dt,
        }
    }

    fn eval_uncached_position(&self, tdb: f64) -> DVec3 {
        if self.times.len() == 1 {
            return self.positions[0];
        }
        let idx = find_interval_index(&self.times, tdb);
        if idx == 0 {
            return self.positions[0];
        }
        if idx == self.times.len() {
            return self.positions[self.times.len() - 1];
        }
        match self.interpolation {
            TrajectoryInterpolation::Linear => {
                let frac = (tdb - self.times[idx - 1]) / (self.times[idx] - self.times[idx - 1]);
                let a = self.positions[idx - 1];
                let b = self.positions[idx];
                DVec3::new(
                    lerp(a.x, b.x, frac),
                    lerp(a.y, b.y, frac),
                    lerp(a.z, b.z, frac),
                )
            }
            TrajectoryInterpolation::Cubic => self.hermite_segment(tdb, idx).eval_position(),
        }
    }
}

impl Orbit for SampledOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        if let Ok(guard) = self.last_eval.lock()
            && let Some((cached_t, cached_pos)) = *guard
            && cached_t.to_bits() == tdb.to_bits()
        {
            return cached_pos;
        }
        let pos = self.eval_uncached_position(tdb);
        if let Ok(mut guard) = self.last_eval.lock() {
            *guard = Some((tdb, pos));
        }
        pos
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        if self.times.len() < 2 {
            return DVec3::ZERO;
        }
        let idx = find_interval_index(&self.times, tdb);
        if idx == 0 || idx == self.times.len() {
            return DVec3::ZERO;
        }
        match self.interpolation {
            TrajectoryInterpolation::Linear => {
                let dt_recip = 1.0 / (self.times[idx] - self.times[idx - 1]);
                (self.positions[idx] - self.positions[idx - 1]) * dt_recip
            }
            TrajectoryInterpolation::Cubic => self.hermite_segment(tdb, idx).eval_velocity(),
        }
    }

    fn period(&self) -> f64 {
        self.times[self.times.len() - 1] - self.times[0]
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn is_periodic(&self) -> bool {
        false
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.times[0], self.times[self.times.len() - 1])
    }

    fn sample(&self, _start_time: f64, _end_time: f64) -> Vec<OrbitSample> {
        let len = self.times.len();
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let pos = self.positions[i];
            let vel = if len == 1 {
                DVec3::ZERO
            } else if i == 0 {
                (self.positions[1] - pos) / (self.times[1] - self.times[0])
            } else if i == len - 1 {
                (pos - self.positions[i - 1]) / (self.times[i] - self.times[i - 1])
            } else {
                let v0 = (self.positions[i + 1] - pos) / (self.times[i + 1] - self.times[i]);
                let v1 = (pos - self.positions[i - 1]) / (self.times[i] - self.times[i - 1]);
                (v0 + v1) * 0.5
            };
            out.push(OrbitSample {
                time: self.times[i],
                position: pos,
                velocity: vel,
            });
        }
        out
    }
}

/// Unevenly spaced position+velocity trajectory (`.xyzv` and `.xyzvbin`) with Hermite cubic
/// or linear interpolation.
#[derive(Debug)]
pub struct SampledOrbitXyzv {
    times: Vec<f64>,
    positions: Vec<DVec3>,
    velocities: Vec<DVec3>,
    interpolation: TrajectoryInterpolation,
    bounding_radius: f64,
    last_eval: Mutex<Option<(f64, DVec3)>>,
}

impl SampledOrbitXyzv {
    /// Parse an ASCII `.xyzv` trajectory (`tdb x y z vx vy vz`, velocities in `km/s`).
    ///
    /// # Errors
    /// Returns [`SampleError::MalformedToken`] for malformed/non-finite fields or trailing tokens,
    /// or [`SampleError::EmptySamples`] if no monotonic sample exists.
    pub fn from_xyzv_str(
        input: &str,
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        let tokens = collect_ascii_tokens(input);
        let mut iter = tokens.chunks_exact(7);
        let mut times = Vec::new();
        let mut positions = Vec::new();
        let mut velocities = Vec::new();
        let mut last_time = f64::NEG_INFINITY;

        for chunk in &mut iter {
            let tdb = parse_finite_f64(chunk[0].0, chunk[0].1)?;
            let pos = DVec3::new(
                parse_finite_f64(chunk[1].0, chunk[1].1)?,
                parse_finite_f64(chunk[2].0, chunk[2].1)?,
                parse_finite_f64(chunk[3].0, chunk[3].1)?,
            );
            let vel = DVec3::new(
                parse_finite_f64(chunk[4].0, chunk[4].1)?,
                parse_finite_f64(chunk[5].0, chunk[5].1)?,
                parse_finite_f64(chunk[6].0, chunk[6].1)?,
            );
            if tdb <= last_time {
                continue;
            }
            last_time = tdb;
            times.push(tdb);
            positions.push(to_celestia_coords(pos));
            velocities.push(to_celestia_coords(vel * SECONDS_PER_DAY));
        }

        if let Some(&(line, _)) = iter.remainder().first() {
            return Err(SampleError::MalformedToken { line });
        }

        Self::from_vectors(times, positions, velocities, interpolation)
    }

    /// Decode a binary `CELXYZV\0` (`.xyzvbin`) trajectory supporting both little-endian (`1234`)
    /// and big-endian (`4321`) IEEE-754 payloads.
    ///
    /// # Errors
    /// Returns [`SampleError::InvalidHeader`] on header mismatch, [`SampleError::Binary`] on EOF,
    /// or [`SampleError::EmptySamples`] if zero monotonic records remain.
    pub fn from_xyzvbin(
        bytes: &[u8],
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        let mut reader = BinaryReader::new(bytes);
        let magic = reader.read_array::<8>()?;
        if &magic != XYZV_MAGIC {
            return Err(SampleError::InvalidHeader("bad CELXYZV magic"));
        }

        // Probe byte order encoded in the payload's own endianness.
        let order_bytes = reader.read_array::<2>()?;
        let le_order = u16::from_le_bytes(order_bytes);
        let be_order = u16::from_be_bytes(order_bytes);
        let little_endian = if le_order == XYZV_LITTLE_ENDIAN {
            true
        } else if be_order == XYZV_BIG_ENDIAN {
            false
        } else {
            return Err(SampleError::InvalidHeader("unsupported byte order"));
        };

        let digits = read_u16(&mut reader, little_endian)?;
        if digits != XYZV_DOUBLE_DIGITS {
            return Err(SampleError::InvalidHeader("unsupported double digits"));
        }
        let _reserved = read_u32(&mut reader, little_endian)?;
        let count_u64 = read_u64(&mut reader, little_endian)?;
        if count_u64 == 0 {
            return Err(SampleError::InvalidHeader("zero record count"));
        }
        let count = usize::try_from(count_u64)
            .map_err(|_| SampleError::InvalidHeader("record count exceeds usize"))?;

        let mut times = Vec::with_capacity(count);
        let mut positions = Vec::with_capacity(count);
        let mut velocities = Vec::with_capacity(count);
        let mut last_time = f64::NEG_INFINITY;

        for idx in 0..count {
            let tdb = read_f64(&mut reader, little_endian)?;
            let pos = DVec3::new(
                read_f64(&mut reader, little_endian)?,
                read_f64(&mut reader, little_endian)?,
                read_f64(&mut reader, little_endian)?,
            );
            let vel = DVec3::new(
                read_f64(&mut reader, little_endian)?,
                read_f64(&mut reader, little_endian)?,
                read_f64(&mut reader, little_endian)?,
            );
            if !tdb.is_finite() || !pos.is_finite() || !vel.is_finite() {
                return Err(SampleError::MalformedToken { line: idx + 1 });
            }
            if tdb <= last_time {
                continue;
            }
            last_time = tdb;
            times.push(tdb);
            positions.push(to_celestia_coords(pos));
            velocities.push(to_celestia_coords(vel * SECONDS_PER_DAY));
        }

        Self::from_vectors(times, positions, velocities, interpolation)
    }

    /// Encode this trajectory as a `CELXYZV\0` binary payload in little- or big-endian byte order.
    #[must_use]
    pub fn to_xyzvbin(&self, little_endian: bool) -> Vec<u8> {
        let mut writer = BinaryWriter::new();
        writer.write_bytes(XYZV_MAGIC);
        let order = if little_endian {
            XYZV_LITTLE_ENDIAN
        } else {
            XYZV_BIG_ENDIAN
        };
        write_u16(&mut writer, order, little_endian);
        write_u16(&mut writer, XYZV_DOUBLE_DIGITS, little_endian);
        write_u32(&mut writer, 0, little_endian);
        let count = u64::try_from(self.times.len()).unwrap_or(0);
        write_u64(&mut writer, count, little_endian);

        for i in 0..self.times.len() {
            write_f64(&mut writer, self.times[i], little_endian);
            let pos = from_celestia_coords(self.positions[i]);
            let vel = from_celestia_coords(self.velocities[i]) / SECONDS_PER_DAY;
            write_f64(&mut writer, pos.x, little_endian);
            write_f64(&mut writer, pos.y, little_endian);
            write_f64(&mut writer, pos.z, little_endian);
            write_f64(&mut writer, vel.x, little_endian);
            write_f64(&mut writer, vel.y, little_endian);
            write_f64(&mut writer, vel.z, little_endian);
        }
        writer.into_inner()
    }

    /// Construct directly from [`XyzvSample`] records already in Celestia coords (`km`, `km/day`).
    ///
    /// # Errors
    /// Returns [`SampleError::EmptySamples`] if zero monotonic finite samples remain.
    pub fn from_samples(
        samples: &[XyzvSample],
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        let mut times = Vec::with_capacity(samples.len());
        let mut positions = Vec::with_capacity(samples.len());
        let mut velocities = Vec::with_capacity(samples.len());
        let mut last_time = f64::NEG_INFINITY;

        for sample in samples {
            if !sample.time.is_finite()
                || !sample.position.is_finite()
                || !sample.velocity.is_finite()
            {
                continue;
            }
            if sample.time <= last_time {
                continue;
            }
            last_time = sample.time;
            times.push(sample.time);
            positions.push(sample.position);
            velocities.push(sample.velocity);
        }

        Self::from_vectors(times, positions, velocities, interpolation)
    }

    fn from_vectors(
        times: Vec<f64>,
        positions: Vec<DVec3>,
        velocities: Vec<DVec3>,
        interpolation: TrajectoryInterpolation,
    ) -> Result<Self, SampleError> {
        if times.is_empty() || times.len() != positions.len() || times.len() != velocities.len() {
            return Err(SampleError::EmptySamples);
        }
        let bounding_radius = compute_bounding_radius(positions.iter().copied());
        Ok(Self {
            times,
            positions,
            velocities,
            interpolation,
            bounding_radius,
            last_eval: Mutex::new(None),
        })
    }

    /// Number of retained monotonic samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether the sample table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    fn hermite_segment(&self, tdb: f64, idx: usize) -> HermiteSegment {
        let dt = self.times[idx] - self.times[idx - 1];
        let inv_dt = 1.0 / dt;
        let frac = (tdb - self.times[idx - 1]) * inv_dt;
        HermiteSegment {
            pos0: self.positions[idx - 1],
            vel0: self.velocities[idx - 1] * dt,
            pos1: self.positions[idx],
            vel1: self.velocities[idx] * dt,
            frac,
            inv_dt,
        }
    }

    fn eval_uncached_position(&self, tdb: f64) -> DVec3 {
        if self.times.len() == 1 {
            return self.positions[0];
        }
        let idx = find_interval_index(&self.times, tdb);
        if idx == 0 {
            return self.positions[0];
        }
        if idx == self.times.len() {
            return self.positions[self.times.len() - 1];
        }
        match self.interpolation {
            TrajectoryInterpolation::Linear => {
                let frac = (tdb - self.times[idx - 1]) / (self.times[idx] - self.times[idx - 1]);
                let a = self.positions[idx - 1];
                let b = self.positions[idx];
                a + (b - a) * frac
            }
            TrajectoryInterpolation::Cubic => self.hermite_segment(tdb, idx).eval_position(),
        }
    }
}

impl Orbit for SampledOrbitXyzv {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        if let Ok(guard) = self.last_eval.lock()
            && let Some((cached_t, cached_pos)) = *guard
            && cached_t.to_bits() == tdb.to_bits()
        {
            return cached_pos;
        }
        let pos = self.eval_uncached_position(tdb);
        if let Ok(mut guard) = self.last_eval.lock() {
            *guard = Some((tdb, pos));
        }
        pos
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        if self.times.len() < 2 {
            return DVec3::ZERO;
        }
        let idx = find_interval_index(&self.times, tdb);
        if idx == 0 || idx == self.times.len() {
            return DVec3::ZERO;
        }
        match self.interpolation {
            TrajectoryInterpolation::Linear => {
                let dt_recip = 1.0 / (self.times[idx] - self.times[idx - 1]);
                (self.positions[idx] - self.positions[idx - 1]) * dt_recip
            }
            TrajectoryInterpolation::Cubic => self.hermite_segment(tdb, idx).eval_velocity(),
        }
    }

    fn period(&self) -> f64 {
        self.times[self.times.len() - 1] - self.times[0]
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn is_periodic(&self) -> bool {
        false
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.times[0], self.times[self.times.len() - 1])
    }

    fn sample(&self, _start_time: f64, _end_time: f64) -> Vec<OrbitSample> {
        self.times
            .iter()
            .zip(self.positions.iter().zip(self.velocities.iter()))
            .map(|(&time, (&position, &velocity))| OrbitSample {
                time,
                position,
                velocity,
            })
            .collect()
    }
}

/// Attitude rotation model interpolating timestamped quaternion keyframes (`.q`) via `SLERP`.
#[derive(Clone, Debug, PartialEq)]
pub struct SampledOrientation {
    times: Vec<f64>,
    rotations: Vec<DQuat>,
}

impl SampledOrientation {
    /// Parse an ASCII `.q` orientation file (`tdb qw qx qy qz`, with optional `#` comments).
    ///
    /// Matches `samporient.cpp`: normalizes each quaternion and multiplies by `XRot90`
    /// (`x_rotation(FRAC_PI_2)`) to transform into Celestia's internal coordinate system.
    ///
    /// # Errors
    /// Returns [`SampleError::MalformedToken`], [`SampleError::InvalidQuaternion`], or
    /// [`SampleError::EmptySamples`].
    pub fn from_q_str(input: &str) -> Result<Self, SampleError> {
        let tokens = collect_ascii_tokens(input);
        let mut iter = tokens.chunks_exact(5);
        let mut times = Vec::new();
        let mut rotations = Vec::new();
        let mut last_time = f64::NEG_INFINITY;
        let x_rot_90 = x_rotation(FRAC_PI_2);

        for chunk in &mut iter {
            let tdb = parse_finite_f64(chunk[0].0, chunk[0].1)?;
            let qw = parse_finite_f64(chunk[1].0, chunk[1].1)?;
            let qx = parse_finite_f64(chunk[2].0, chunk[2].1)?;
            let qy = parse_finite_f64(chunk[3].0, chunk[3].1)?;
            let qz = parse_finite_f64(chunk[4].0, chunk[4].1)?;
            let raw = DQuat::from_xyzw(qx, qy, qz, qw);
            if !raw.is_finite() || raw.length_squared() <= f64::MIN_POSITIVE {
                return Err(SampleError::InvalidQuaternion);
            }
            if tdb <= last_time {
                continue;
            }
            last_time = tdb;
            times.push(tdb);
            rotations.push(raw.normalize() * x_rot_90);
        }

        if let Some(&(line, _)) = iter.remainder().first() {
            return Err(SampleError::MalformedToken { line });
        }

        Self::from_times_and_rotations(times, rotations)
    }

    /// Construct from already-converted [`OrientationKeyframe`] entries.
    ///
    /// # Errors
    /// Returns [`SampleError::InvalidQuaternion`] if any quaternion is non-finite/zero,
    /// or [`SampleError::EmptySamples`] if zero monotonic keyframes remain.
    pub fn from_keyframes(keyframes: &[OrientationKeyframe]) -> Result<Self, SampleError> {
        let mut times = Vec::with_capacity(keyframes.len());
        let mut rotations = Vec::with_capacity(keyframes.len());
        let mut last_time = f64::NEG_INFINITY;

        for kf in keyframes {
            if !kf.rotation.is_finite() || kf.rotation.length_squared() <= f64::MIN_POSITIVE {
                return Err(SampleError::InvalidQuaternion);
            }
            if !kf.time.is_finite() || kf.time <= last_time {
                continue;
            }
            last_time = kf.time;
            times.push(kf.time);
            rotations.push(kf.rotation.normalize());
        }

        Self::from_times_and_rotations(times, rotations)
    }

    fn from_times_and_rotations(
        times: Vec<f64>,
        rotations: Vec<DQuat>,
    ) -> Result<Self, SampleError> {
        if times.is_empty() || times.len() != rotations.len() {
            return Err(SampleError::EmptySamples);
        }
        Ok(Self { times, rotations })
    }

    /// Number of retained monotonic keyframes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether the keyframe list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }
}

impl RotationModel for SampledOrientation {
    fn spin(&self, tdb: f64) -> DQuat {
        if self.times.len() == 1 {
            return self.rotations[0];
        }
        let idx = find_interval_index(&self.times, tdb);
        if idx == 0 {
            return self.rotations[0];
        }
        if idx == self.times.len() {
            return self.rotations[self.times.len() - 1];
        }
        let frac = (tdb - self.times[idx - 1]) / (self.times[idx] - self.times[idx - 1]);
        self.rotations[idx - 1].slerp(self.rotations[idx], frac)
    }

    fn period(&self) -> f64 {
        self.times[self.times.len() - 1] - self.times[0]
    }

    fn is_periodic(&self) -> bool {
        false
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.times[0], self.times[self.times.len() - 1])
    }
}

fn read_u16(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<u16, BinaryError> {
    if little_endian {
        reader.read_le_u16()
    } else {
        reader.read_be_u16()
    }
}

fn read_u32(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<u32, BinaryError> {
    if little_endian {
        reader.read_le_u32()
    } else {
        reader.read_be_u32()
    }
}

fn read_u64(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<u64, BinaryError> {
    if little_endian {
        reader.read_le_u64()
    } else {
        reader.read_be_u64()
    }
}

fn read_f64(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<f64, BinaryError> {
    if little_endian {
        reader.read_le_f64()
    } else {
        reader.read_be_f64()
    }
}

fn write_u16(writer: &mut BinaryWriter, val: u16, little_endian: bool) {
    if little_endian {
        writer.write_le_u16(val);
    } else {
        writer.write_be_u16(val);
    }
}

fn write_u32(writer: &mut BinaryWriter, val: u32, little_endian: bool) {
    if little_endian {
        writer.write_le_u32(val);
    } else {
        writer.write_be_u32(val);
    }
}

fn write_u64(writer: &mut BinaryWriter, val: u64, little_endian: bool) {
    if little_endian {
        writer.write_le_u64(val);
    } else {
        writer.write_be_u64(val);
    }
}

fn write_f64(writer: &mut BinaryWriter, val: f64, little_endian: bool) {
    if little_endian {
        writer.write_le_f64(val);
    } else {
        writer.write_be_f64(val);
    }
}

#[cfg(test)]
mod tests {
    use core::f64::consts::FRAC_PI_4;

    use alestia_astro::J2000;

    use super::*;

    #[test]
    fn xyz_linear_and_cubic_interpolation_match_celestia_coords() -> Result<(), SampleError> {
        let data = "\
# Voyager-like position trajectory
2451545.0   100.0  200.0  300.0
2451546.0   110.0  220.0  330.0
2451546.0   999.0  999.0  999.0  # duplicate timestamp skipped
2451547.0   120.0  240.0  360.0
2451548.0   130.0  260.0  390.0
";
        let linear = SampledOrbit::from_xyz_str(data, TrajectoryInterpolation::Linear)?;
        let cubic = SampledOrbit::from_xyz_str(data, TrajectoryInterpolation::Cubic)?;

        assert_eq!(linear.len(), 4);
        assert!(!linear.is_periodic());
        assert_eq!(linear.valid_range(), (J2000, J2000 + 3.0));

        // Clamping outside valid range
        let clamped_before = linear.position_at_time(J2000 - 10.0);
        assert!((clamped_before - DVec3::new(100.0, 300.0, -200.0)).length() < 1e-12);
        assert!((linear.velocity_at_time(J2000 - 10.0)).length() < 1e-12);

        let mid = linear.position_at_time(J2000 + 1.5);
        assert!((mid - DVec3::new(115.0, 345.0, -230.0)).length() < 1e-12);
        // Constant slope should preserve exact affine velocity under Catmull-Rom cubic.
        let vel_cubic = cubic.velocity_at_time(J2000 + 1.5);
        assert!((vel_cubic - DVec3::new(10.0, 30.0, -20.0)).length() < 1e-11);
        assert!((cubic.position_at_time(J2000 + 1.5) - mid).length() < 1e-11);
        assert_eq!(cubic.sample(J2000, J2000 + 3.0).len(), 4);
        Ok(())
    }

    #[test]
    fn xyzv_ascii_and_binary_le_be_roundtrip() -> Result<(), SampleError> {
        let xyzv = "\
# TDB x y z vx vy vz (km, km/s)
2451545.0   0.0    0.0    0.0    1.0   0.0   0.0
2451546.0   86400.0 0.0   0.0    1.0   0.0   0.0
";
        let ascii = SampledOrbitXyzv::from_xyzv_str(xyzv, TrajectoryInterpolation::Cubic)?;
        assert!((ascii.position_at_time(J2000 + 0.5).x - 43_200.0).abs() < 1e-8);
        assert!((ascii.velocity_at_time(J2000 + 0.5).x - SECONDS_PER_DAY).abs() < 1e-8);

        for little_endian in [true, false] {
            let bytes = ascii.to_xyzvbin(little_endian);
            let decoded = SampledOrbitXyzv::from_xyzvbin(&bytes, TrajectoryInterpolation::Cubic)?;
            assert!(
                (decoded.position_at_time(J2000 + 0.25) - ascii.position_at_time(J2000 + 0.25))
                    .length()
                    < 1e-10
            );
            assert!(
                (decoded.velocity_at_time(J2000 + 0.25) - ascii.velocity_at_time(J2000 + 0.25))
                    .length()
                    < 1e-10
            );
        }

        let linear = SampledOrbitXyzv::from_samples(
            &[
                XyzvSample {
                    time: J2000,
                    position: DVec3::ZERO,
                    velocity: DVec3::new(100.0, 0.0, 0.0),
                },
                XyzvSample {
                    time: J2000 + 2.0,
                    position: DVec3::new(200.0, 0.0, 0.0),
                    velocity: DVec3::new(100.0, 0.0, 0.0),
                },
            ],
            TrajectoryInterpolation::Linear,
        )?;
        assert!((linear.position_at_time(J2000 + 1.0).x - 100.0).abs() < 1e-12);
        assert!((linear.velocity_at_time(J2000 + 1.0).x - 100.0).abs() < 1e-12);

        let err = SampledOrbitXyzv::from_xyzvbin(b"BADMAGIC", TrajectoryInterpolation::Cubic);
        assert!(matches!(err, Err(SampleError::InvalidHeader(_))));
        Ok(())
    }

    #[test]
    fn sampled_orientation_slerp_and_errors() -> Result<(), SampleError> {
        let cos_h = FRAC_PI_4.cos();
        let sin_h = FRAC_PI_4.sin();
        let q_text = format!(
            "\
# attitude keyframes
2451545.0 1.0 0.0 0.0 0.0
2451546.0 {cos_h} 0.0 0.0 {sin_h}
"
        );
        let orient = SampledOrientation::from_q_str(&q_text)?;
        assert_eq!(orient.len(), 2);
        assert!(!orient.is_periodic());
        assert_eq!(orient.valid_range(), (J2000, J2000 + 1.0));

        let mid = orient.spin(J2000 + 0.5);
        let expected_mid = DQuat::from_rotation_z(FRAC_PI_4) * x_rotation(FRAC_PI_2);
        assert!(mid.angle_between(expected_mid) < 1e-12);

        let trailing =
            SampledOrbit::from_xyz_str("2451545.0 1.0 2.0", TrajectoryInterpolation::Linear);
        assert!(matches!(trailing, Err(SampleError::MalformedToken { .. })));

        let zero_q = SampledOrientation::from_q_str("2451545.0 0.0 0.0 0.0 0.0");
        assert_eq!(zero_q, Err(SampleError::InvalidQuaternion));
        Ok(())
    }
}
