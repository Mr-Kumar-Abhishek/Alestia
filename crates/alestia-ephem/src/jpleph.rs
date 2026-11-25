//! Binary `JPL` ephemeris reader (`DE200`, `DE405`, `DE406`, `DE430`, `DE440`, `INPOP`)
//! and [`JplOrbit`] wrapper matching `celephem/jpleph.cpp`, `jpleph.h`, and `customorbit.cpp`
//! (`FR-012`).

use core::fmt;
use std::sync::Arc;

use alestia_astro::{J2000_OBLIQUITY, SOLAR_MASS};
use alestia_math::{DVec3, x_rotation};
use alestia_util::{BinaryError, BinaryReader};

use crate::orbit::{
    AdaptiveSamplingParams, MixedOrbit, Orbit, OrbitError, OrbitSample, adaptive_sample_orbit,
};

/// Number of ephemeris tables in a standard JPL binary header (`Mercury..Pluto`, `Moon`, `Sun`,
/// and 2-component nutation).
pub const JPL_EPH_N_ITEMS: usize = 12;

/// Maximum Chebyshev polynomial degree coefficients per component supported by upstream Celestia.
pub const MAX_CHEBYSHEV_COEFFS: usize = 32;

const N_CONSTANTS: usize = 400;
const CONSTANT_NAME_LEN: usize = 6;
const LABEL_SIZE: usize = 84;
const HEADER_PREFIX_BYTES: usize = 3 * LABEL_SIZE + N_CONSTANTS * CONSTANT_NAME_LEN;
const DE_NUM_OFFSET: usize = HEADER_PREFIX_BYTES + 3 * 8 + 4 + 2 * 8 + JPL_EPH_N_ITEMS * 12;
const INPOP_DE_COMPATIBLE: u32 = 100;
const MIN_DE_NUMBER: u32 = 200;

/// Target/origin body identifiers stored in JPL Development Ephemeris records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum JplEphemItem {
    /// Mercury barycenter.
    Mercury = 0,
    /// Venus barycenter.
    Venus = 1,
    /// Earth-Moon barycenter.
    EarthMoonBary = 2,
    /// Mars barycenter.
    Mars = 3,
    /// Jupiter barycenter.
    Jupiter = 4,
    /// Saturn barycenter.
    Saturn = 5,
    /// Uranus barycenter.
    Uranus = 6,
    /// Neptune barycenter.
    Neptune = 7,
    /// Pluto barycenter.
    Pluto = 8,
    /// Geocentric Moon.
    Moon = 9,
    /// Sun relative to Solar System Barycenter.
    Sun = 10,
    /// Earth computed from Earth-Moon barycenter and Moon (`EarthMoonBary - Moon / (1 + EMRAT)`).
    Earth = 11,
    /// Solar System Barycenter origin.
    Ssb = 12,
}

/// Chebyshev coefficient block metadata for one JPL ephemeris body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JplCoeffInfo {
    /// 0-based offset into `JplRecord::coeffs` (upstream raw Fortran 1-based word minus 3).
    pub offset: u32,
    /// Number of Chebyshev coefficients per spatial component (`<= 32`).
    pub n_coeffs: u32,
    /// Number of sub-intervals per `days_per_interval` record (or `u32::MAX` sentinel).
    pub n_granules: u32,
}

/// Single time-interval Chebyshev coefficient record in a JPL binary file.
#[derive(Clone, Debug, PartialEq)]
pub struct JplRecord {
    /// Record interval start time in TDB Julian Date.
    pub t0: f64,
    /// Record interval end time in TDB Julian Date.
    pub t1: f64,
    /// Flattened Chebyshev coefficient table (excluding `t0` and `t1`).
    pub coeffs: Vec<f64>,
}

/// Errors returned while decoding or validating a binary JPL ephemeris.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JplEphemerisError {
    /// DE version field at byte `2840` does not match DE200+ or INPOP (`100`).
    InvalidDeNumber,
    /// Header time bounds, constants, or coefficient metadata failed validation.
    InvalidHeader(&'static str),
    /// Binary stream ended unexpectedly.
    Binary(BinaryError),
}

impl fmt::Display for JplEphemerisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeNumber => write!(f, "invalid or unrecognized JPL DE/INPOP number"),
            Self::InvalidHeader(msg) => write!(f, "invalid JPL ephemeris header: {msg}"),
            Self::Binary(err) => write!(f, "JPL ephemeris read error: {err}"),
        }
    }
}

impl std::error::Error for JplEphemerisError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Binary(err) => Some(err),
            _ => None,
        }
    }
}

impl From<BinaryError> for JplEphemerisError {
    fn from(err: BinaryError) -> Self {
        Self::Binary(err)
    }
}

/// Decoded JPL Development Ephemeris (`DE200`, `DE405`, `DE406`, `DE430`, `DE440`, `INPOP`) table.
#[derive(Clone, Debug)]
pub struct JplEphemeris {
    coeff_info: [JplCoeffInfo; JPL_EPH_N_ITEMS],
    start_date: f64,
    end_date: f64,
    days_per_interval: f64,
    au_km: f64,
    earth_moon_mass_ratio: f64,
    de_num: u32,
    record_size: u32,
    little_endian: bool,
    records: Vec<JplRecord>,
}

impl JplEphemeris {
    /// Decode a JPL binary ephemeris from raw bytes with automatic endianness detection.
    ///
    /// # Errors
    /// Returns [`JplEphemerisError`] if the DE/INPOP header, endianness probe, coefficient bounds,
    /// or Chebyshev records are malformed or truncated.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JplEphemerisError> {
        let (de_num, little_endian) = detect_endianness(bytes)?;
        let mut reader = BinaryReader::new(bytes);
        let _header_bytes = reader.read_bytes(HEADER_PREFIX_BYTES)?;

        let start_date = read_f64(&mut reader, little_endian)?;
        let end_date = read_f64(&mut reader, little_endian)?;
        let days_per_interval = read_f64(&mut reader, little_endian)?;
        if !start_date.is_finite()
            || !end_date.is_finite()
            || !days_per_interval.is_finite()
            || end_date <= start_date
            || days_per_interval <= 0.0
        {
            return Err(JplEphemerisError::InvalidHeader(
                "invalid ephemeris time interval",
            ));
        }

        let _n_constants = read_u32(&mut reader, little_endian)?;
        let au_km = read_f64(&mut reader, little_endian)?;
        let earth_moon_mass_ratio = read_f64(&mut reader, little_endian)?;
        if !au_km.is_finite()
            || au_km <= 0.0
            || !earth_moon_mass_ratio.is_finite()
            || earth_moon_mass_ratio <= 0.0
        {
            return Err(JplEphemerisError::InvalidHeader(
                "invalid AU or Earth-Moon mass ratio",
            ));
        }

        let (coeff_info, mut record_size) = read_coeff_infos(&mut reader, little_endian)?;
        let _de_again = read_u32(&mut reader, little_endian)?;
        let libration_coeff_info = read_coeff_entry(&mut reader, little_endian)?;

        let lib_words = libration_coeff_info
            .n_coeffs
            .checked_mul(libration_coeff_info.n_granules)
            .and_then(|v| v.checked_mul(3))
            .ok_or(JplEphemerisError::InvalidHeader(
                "libration record overflow",
            ))?;
        record_size = record_size
            .checked_add(lib_words)
            .and_then(|v| v.checked_add(2))
            .ok_or(JplEphemerisError::InvalidHeader("record size overflow"))?;

        if de_num == INPOP_DE_COMPATIBLE {
            record_size = read_u32(&mut reader, little_endian)?;
        }
        if record_size < 2 {
            return Err(JplEphemerisError::InvalidHeader("record size < 2"));
        }

        let rec_len = usize::try_from(record_size)
            .map_err(|_| JplEphemerisError::InvalidHeader("record size overflow"))?;
        validate_planet_coeffs(&coeff_info, rec_len - 2)?;

        let records_offset = rec_len
            .checked_mul(2)
            .and_then(|w| w.checked_mul(8))
            .ok_or(JplEphemerisError::InvalidHeader("records offset overflow"))?;
        if bytes.len() < records_offset {
            return Err(JplEphemerisError::Binary(BinaryError::UnexpectedEof {
                needed: records_offset,
                remaining: bytes.len(),
            }));
        }

        let n_records_f = ((end_date - start_date) / days_per_interval).floor();
        let n_records = f64_to_non_zero_usize(n_records_f)?;

        let mut rec_reader = BinaryReader::new(&bytes[records_offset..]);
        let mut records = Vec::with_capacity(n_records);
        let coeff_len = rec_len - 2;
        for _ in 0..n_records {
            let t0 = read_f64(&mut rec_reader, little_endian)?;
            let t1 = read_f64(&mut rec_reader, little_endian)?;
            let mut coeffs = Vec::with_capacity(coeff_len);
            for _ in 0..coeff_len {
                coeffs.push(read_f64(&mut rec_reader, little_endian)?);
            }
            records.push(JplRecord { t0, t1, coeffs });
        }

        Ok(Self {
            coeff_info,
            start_date,
            end_date,
            days_per_interval,
            au_km,
            earth_moon_mass_ratio,
            de_num,
            record_size,
            little_endian,
            records,
        })
    }

    /// Ephemeris DE release number (`200`, `405`, `406`, `430`, `440`, or `100` for INPOP).
    #[must_use]
    pub const fn de_number(&self) -> u32 {
        self.de_num
    }

    /// Ephemeris coverage start time in TDB Julian Date.
    #[must_use]
    pub const fn start_date(&self) -> f64 {
        self.start_date
    }

    /// Ephemeris coverage end time in TDB Julian Date.
    #[must_use]
    pub const fn end_date(&self) -> f64 {
        self.end_date
    }

    /// Time span per Chebyshev record in days.
    #[must_use]
    pub const fn days_per_interval(&self) -> f64 {
        self.days_per_interval
    }

    /// Kilometers per astronomical unit stored in header.
    #[must_use]
    pub const fn au_km(&self) -> f64 {
        self.au_km
    }

    /// Earth-to-Moon mass ratio stored in header.
    #[must_use]
    pub const fn earth_moon_mass_ratio(&self) -> f64 {
        self.earth_moon_mass_ratio
    }

    /// Number of `f64` doubles per ephemeris record (including `t0` and `t1`).
    #[must_use]
    pub const fn record_size(&self) -> u32 {
        self.record_size
    }

    /// `true` if decoded binary payload is little-endian; `false` for big-endian.
    #[must_use]
    pub const fn is_little_endian(&self) -> bool {
        self.little_endian
    }

    /// Number of loaded Chebyshev records.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Evaluate the ICRF J2000 equatorial position vector (in km) relative to the Solar System
    /// Barycenter (or relative to Earth when `item == JplEphemItem::Moon`) at TDB Julian Date `tdb`.
    ///
    /// Out-of-range timestamps are clamped to `[start_date, end_date]` matching `jpleph.cpp`.
    #[must_use]
    pub fn planet_position(&self, item: JplEphemItem, tdb: f64) -> DVec3 {
        match item {
            JplEphemItem::Ssb => DVec3::ZERO,
            JplEphemItem::Earth => {
                let emb_pos = self.planet_position(JplEphemItem::EarthMoonBary, tdb);
                let moon_pos = self.planet_position(JplEphemItem::Moon, tdb);
                emb_pos - moon_pos * (1.0 / (self.earth_moon_mass_ratio + 1.0))
            }
            _ => self.eval_table_item(item as usize, tdb),
        }
    }

    fn eval_table_item(&self, item_idx: usize, raw_tdb: f64) -> DVec3 {
        let tjd = raw_tdb.clamp(self.start_date, self.end_date);
        let rec_idx = self
            .records
            .partition_point(|rec| rec.t1 <= tjd)
            .min(self.records.len() - 1);
        let rec = &self.records[rec_idx];
        let info = self.coeff_info[item_idx];
        let n_coeffs = usize::try_from(info.n_coeffs).unwrap_or(0);
        if !(2..=MAX_CHEBYSHEV_COEFFS).contains(&n_coeffs) {
            return DVec3::ZERO;
        }
        let base_offset = usize::try_from(info.offset).unwrap_or(0);

        let (coeff_start, param_u) = if info.n_granules == u32::MAX {
            (
                base_offset,
                2.0 * (tjd - rec.t0) / self.days_per_interval - 1.0,
            )
        } else {
            let days_per_granule = self.days_per_interval / f64::from(info.n_granules);
            let mut granule = 0_u32;
            for candidate in 1..info.n_granules {
                let boundary = rec.t0 + days_per_granule * f64::from(candidate);
                if tjd >= boundary {
                    granule = candidate;
                } else {
                    break;
                }
            }
            let granule_start = rec.t0 + days_per_granule * f64::from(granule);
            let granule_usize = usize::try_from(granule).unwrap_or(0);
            let start = base_offset + granule_usize * n_coeffs * 3;
            let u = 2.0 * (tjd - granule_start) / days_per_granule - 1.0;
            (start, u)
        };

        let mut sums = [0.0_f64; 3];
        let mut cheb = [0.0_f64; MAX_CHEBYSHEV_COEFFS];
        cheb[0] = 1.0;
        cheb[1] = param_u;
        for (axis, sum_slot) in sums.iter_mut().enumerate() {
            let axis_off = coeff_start + axis * n_coeffs;
            let slice = rec.coeffs.get(axis_off..axis_off + n_coeffs);
            let Some(c) = slice else {
                return DVec3::ZERO;
            };
            let mut acc = c[0] + c[1] * param_u;
            for j in 2..n_coeffs {
                cheb[j] = 2.0 * param_u * cheb[j - 1] - cheb[j - 2];
                acc += c[j] * cheb[j];
            }
            *sum_slot = acc;
        }
        DVec3::new(sums[0], sums[1], sums[2])
    }
}

/// [`Orbit`] adapter evaluating a target relative to a center body in Celestia ecliptic `km`.
#[derive(Clone, Debug)]
pub struct JplOrbit {
    ephem: Arc<JplEphemeris>,
    target: JplEphemItem,
    center: JplEphemItem,
    period: f64,
    bounding_radius: f64,
}

impl JplOrbit {
    /// Construct a [`JplOrbit`] evaluating `target` relative to `center`.
    #[must_use]
    pub fn new(
        ephem: Arc<JplEphemeris>,
        target: JplEphemItem,
        center: JplEphemItem,
        period: f64,
        bounding_radius: f64,
    ) -> Self {
        Self {
            ephem,
            target,
            center,
            period,
            bounding_radius,
        }
    }

    /// Wrap this [`JplOrbit`] inside a [`MixedOrbit`] clamped to the JPL coverage window with
    /// Keplerian extrapolation outside `[start_date, end_date]` (matching `CreateJPLEphOrbit`).
    ///
    /// # Errors
    /// Propagates [`OrbitError`] if the underlying JPL file time span is invalid.
    pub fn into_mixed_orbit(self) -> Result<MixedOrbit, OrbitError> {
        let begin = self.ephem.start_date();
        let end = self.ephem.end_date();
        MixedOrbit::new(Arc::new(self), begin, end, SOLAR_MASS)
    }
}

impl Orbit for JplOrbit {
    fn position_at_time(&self, tdb: f64) -> DVec3 {
        let mut pos = self.ephem.planet_position(self.target, tdb);
        let direct_ssb = self.center == JplEphemItem::Ssb && self.target != JplEphemItem::Moon;
        let direct_lunar = self.center == JplEphemItem::Earth && self.target == JplEphemItem::Moon;
        if !direct_ssb && !direct_lunar {
            let mut center_pos = self.ephem.planet_position(self.center, tdb);
            if self.target == JplEphemItem::Moon {
                pos += self.ephem.planet_position(JplEphemItem::Earth, tdb);
            }
            if self.center == JplEphemItem::Moon {
                center_pos += self.ephem.planet_position(JplEphemItem::Earth, tdb);
            }
            pos -= center_pos;
        }
        let ecliptic = x_rotation(-J2000_OBLIQUITY) * pos;
        DVec3::new(ecliptic.x, ecliptic.z, -ecliptic.y)
    }

    fn velocity_at_time(&self, tdb: f64) -> DVec3 {
        const DIFF_STEP: f64 = 1.0 / 1440.0;
        let p_plus = self.position_at_time(tdb + DIFF_STEP);
        let p_minus = self.position_at_time(tdb - DIFF_STEP);
        (p_plus - p_minus) / (2.0 * DIFF_STEP)
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn bounding_radius(&self) -> f64 {
        self.bounding_radius
    }

    fn valid_range(&self) -> (f64, f64) {
        (self.ephem.start_date(), self.ephem.end_date())
    }

    fn sample(&self, start_time: f64, end_time: f64) -> Vec<OrbitSample> {
        adaptive_sample_orbit(
            self,
            start_time,
            end_time,
            AdaptiveSamplingParams::for_span(end_time - start_time),
        )
    }
}

fn detect_endianness(bytes: &[u8]) -> Result<(u32, bool), JplEphemerisError> {
    if bytes.len() < DE_NUM_OFFSET + 4 {
        return Err(JplEphemerisError::Binary(BinaryError::UnexpectedEof {
            needed: DE_NUM_OFFSET + 4,
            remaining: bytes.len(),
        }));
    }
    let slice = [
        bytes[DE_NUM_OFFSET],
        bytes[DE_NUM_OFFSET + 1],
        bytes[DE_NUM_OFFSET + 2],
        bytes[DE_NUM_OFFSET + 3],
    ];
    let le = u32::from_le_bytes(slice);
    let be = u32::from_be_bytes(slice);
    if le == INPOP_DE_COMPATIBLE {
        Ok((le, true))
    } else if be == INPOP_DE_COMPATIBLE {
        Ok((be, false))
    } else if (MIN_DE_NUMBER..=(1_u32 << 15)).contains(&le) {
        Ok((le, true))
    } else if (MIN_DE_NUMBER..=(1_u32 << 15)).contains(&be) {
        Ok((be, false))
    } else {
        Err(JplEphemerisError::InvalidDeNumber)
    }
}

fn read_coeff_entry(
    reader: &mut BinaryReader<'_>,
    little_endian: bool,
) -> Result<JplCoeffInfo, JplEphemerisError> {
    Ok(JplCoeffInfo {
        offset: read_u32(reader, little_endian)?,
        n_coeffs: read_u32(reader, little_endian)?,
        n_granules: read_u32(reader, little_endian)?,
    })
}

fn read_coeff_infos(
    reader: &mut BinaryReader<'_>,
    little_endian: bool,
) -> Result<([JplCoeffInfo; JPL_EPH_N_ITEMS], u32), JplEphemerisError> {
    let mut infos = [JplCoeffInfo {
        offset: 0,
        n_coeffs: 0,
        n_granules: 0,
    }; JPL_EPH_N_ITEMS];
    let mut record_size = 0_u32;

    for (idx, slot) in infos.iter_mut().enumerate() {
        let mut entry = read_coeff_entry(reader, little_endian)?;
        if entry.offset < 3 {
            return Err(JplEphemerisError::InvalidHeader("coefficient offset < 3"));
        }
        entry.offset -= 3;
        let components: u32 = if idx == JPL_EPH_N_ITEMS - 1 { 2 } else { 3 };
        let item_words = entry
            .n_coeffs
            .checked_mul(entry.n_granules)
            .and_then(|v| v.checked_mul(components))
            .ok_or(JplEphemerisError::InvalidHeader(
                "coefficient record size overflow",
            ))?;
        record_size = record_size
            .checked_add(item_words)
            .ok_or(JplEphemerisError::InvalidHeader("record size overflow"))?;
        *slot = entry;
    }

    Ok((infos, record_size))
}

fn validate_planet_coeffs(
    coeff_info: &[JplCoeffInfo; JPL_EPH_N_ITEMS],
    coeff_len: usize,
) -> Result<(), JplEphemerisError> {
    for info in &coeff_info[..11] {
        let n_coeffs = usize::try_from(info.n_coeffs)
            .map_err(|_| JplEphemerisError::InvalidHeader("n_coeffs overflow"))?;
        if n_coeffs > MAX_CHEBYSHEV_COEFFS {
            return Err(JplEphemerisError::InvalidHeader(
                "n_coeffs exceeds MAX_CHEBYSHEV_COEFFS",
            ));
        }
        if n_coeffs == 0 {
            continue;
        }
        let effective_granules = if info.n_granules == u32::MAX {
            1_usize
        } else if (1..=32).contains(&info.n_granules) {
            usize::try_from(info.n_granules)
                .map_err(|_| JplEphemerisError::InvalidHeader("n_granules overflow"))?
        } else {
            return Err(JplEphemerisError::InvalidHeader("invalid granule count"));
        };
        let offset = usize::try_from(info.offset)
            .map_err(|_| JplEphemerisError::InvalidHeader("offset overflow"))?;
        let end = offset
            .checked_add(effective_granules * n_coeffs * 3)
            .ok_or(JplEphemerisError::InvalidHeader("coeff end overflow"))?;
        if end > coeff_len {
            return Err(JplEphemerisError::InvalidHeader(
                "coefficient block extends past record",
            ));
        }
    }
    Ok(())
}

fn f64_to_non_zero_usize(value: f64) -> Result<usize, JplEphemerisError> {
    if !value.is_finite() || value < 1.0 {
        return Err(JplEphemerisError::InvalidHeader(
            "zero or invalid record count",
        ));
    }
    let mut low = 1_usize;
    let mut high = 1_000_000_usize;
    if value > 1_000_000.0 {
        return Err(JplEphemerisError::InvalidHeader("record count too large"));
    }
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let mid_u32 = u32::try_from(mid).unwrap_or(u32::MAX);
        if f64::from(mid_u32) <= value {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    Ok(low)
}

fn read_u32(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<u32, BinaryError> {
    if little_endian {
        reader.read_le_u32()
    } else {
        reader.read_be_u32()
    }
}

fn read_f64(reader: &mut BinaryReader<'_>, little_endian: bool) -> Result<f64, BinaryError> {
    if little_endian {
        reader.read_le_f64()
    } else {
        reader.read_be_f64()
    }
}

#[cfg(test)]
mod tests {
    use alestia_astro::{J2000, KM_PER_AU};
    use alestia_util::BinaryWriter;

    use super::*;

    fn push_u32(w: &mut BinaryWriter, v: u32, le: bool) {
        if le {
            w.write_le_u32(v);
        } else {
            w.write_be_u32(v);
        }
    }

    fn push_f64(w: &mut BinaryWriter, v: f64, le: bool) {
        if le {
            w.write_le_f64(v);
        } else {
            w.write_be_f64(v);
        }
    }

    fn build_synthetic_de440(le: bool) -> Vec<u8> {
        // Construct a single 32-day record with quadratic Chebyshev polynomials for EarthMoonBary
        // and Moon, and linear for Sun, zero-padded to a full record size.
        let mut w = BinaryWriter::new();
        w.write_bytes(&[b' '; 3 * LABEL_SIZE]);
        w.write_bytes(&[b' '; N_CONSTANTS * CONSTANT_NAME_LEN]);
        push_f64(&mut w, J2000, le);
        push_f64(&mut w, J2000 + 32.0, le);
        push_f64(&mut w, 32.0, le);
        push_u32(&mut w, 0, le);
        push_f64(&mut w, KM_PER_AU, le);
        push_f64(&mut w, 81.300_56, le);

        // Give each of the first 11 items 1 granule of 3 coefficients (3*3=9 words per planet),
        // and nutation (idx 11) 1 granule of 2 coeffs (2*2=4 words), 1-based offset starting at 3.
        let mut cur_offset = 3_u32;
        for i in 0..11_u32 {
            push_u32(&mut w, cur_offset, le);
            push_u32(&mut w, 3, le);
            push_u32(&mut w, 1, le);
            cur_offset += 9;
            let _ = i;
        }
        // Item 11 (nutation, 2 components)
        push_u32(&mut w, cur_offset, le);
        push_u32(&mut w, 2, le);
        push_u32(&mut w, 1, le);
        cur_offset += 4;
        // DE number = 440
        push_u32(&mut w, 440, le);
        // Libration info: 300 coeffs x 1 granule x 3 comps keeps the record larger than the header,
        // as in real ephemeris files where the header fits inside the first two records.
        push_u32(&mut w, cur_offset, le);
        push_u32(&mut w, 300, le);
        push_u32(&mut w, 1, le);

        // Record size = 2 (t0,t1) + 11*9 + 4 + 900 libration = 1005 doubles.
        let record_size = 1005_usize;
        let rec_bytes = record_size * 8;
        while w.as_slice().len() < 2 * rec_bytes {
            w.write_u8(0);
        }

        // Write Record 0: t0 = J2000, t1 = J2000 + 32.0
        push_f64(&mut w, J2000, le);
        push_f64(&mut w, J2000 + 32.0, le);

        let mut coeffs = vec![0.0_f64; record_size - 2];
        // EMB (item 2 -> coeff_offset = 2 * 9 = 18): X = 1.0 * AU + 0.01*AU * T1(u) + 0.002*AU * T2(u)
        coeffs[18] = KM_PER_AU;
        coeffs[19] = 0.01 * KM_PER_AU;
        coeffs[20] = 0.002 * KM_PER_AU;
        // Moon (item 9 -> coeff_offset = 9 * 9 = 81): Y = 384_400.0
        coeffs[81 + 3] = 384_400.0;
        // Sun (item 10 -> coeff_offset = 10 * 9 = 90): X = -500_000.0
        coeffs[90] = -500_000.0;

        for c in coeffs {
            push_f64(&mut w, c, le);
        }
        w.into_inner()
    }

    #[test]
    fn decodes_le_and_be_jpl_and_evaluates_chebyshev_and_earth_moon()
    -> Result<(), Box<dyn std::error::Error>> {
        for little_endian in [true, false] {
            let bytes = build_synthetic_de440(little_endian);
            let eph = Arc::new(JplEphemeris::from_bytes(&bytes)?);
            assert_eq!(eph.de_number(), 440);
            assert_eq!(eph.is_little_endian(), little_endian);
            assert_eq!(eph.record_count(), 1);
            assert_eq!(eph.planet_position(JplEphemItem::Ssb, J2000), DVec3::ZERO);

            // At midpoint J2000 + 16, u = 0 => T0(0)=1, T1(0)=0, T2(0)=-1 => (1.0 - 0.002) * AU.
            let emb_mid = eph.planet_position(JplEphemItem::EarthMoonBary, J2000 + 16.0);
            let expected_x = 0.998 * KM_PER_AU;
            assert!((emb_mid.x - expected_x).abs() < 1e-12 * KM_PER_AU);

            let earth_mid = eph.planet_position(JplEphemItem::Earth, J2000 + 16.0);
            let moon_mid = eph.planet_position(JplEphemItem::Moon, J2000 + 16.0);
            let expected_y = -moon_mid.y / (1.0 + eph.earth_moon_mass_ratio());
            assert!((earth_mid.x - expected_x).abs() < 1e-12 * KM_PER_AU);
            assert!((earth_mid.y - expected_y).abs() < 1e-9);

            let orbit = JplOrbit::new(
                Arc::clone(&eph),
                JplEphemItem::Earth,
                JplEphemItem::Sun,
                365.25,
                2.0 * KM_PER_AU,
            );
            let ecl_pos = orbit.position_at_time(J2000 + 16.0);
            assert!(ecl_pos.is_finite());
            assert!(ecl_pos.length() > 0.9 * KM_PER_AU);

            let mixed = orbit.into_mixed_orbit()?;
            assert!(mixed.position_at_time(J2000 + 60.0).is_finite());
        }

        let bad = JplEphemeris::from_bytes(&[0_u8; DE_NUM_OFFSET + 8]);
        assert!(matches!(bad, Err(JplEphemerisError::InvalidDeNumber)));
        Ok(())
    }
}
