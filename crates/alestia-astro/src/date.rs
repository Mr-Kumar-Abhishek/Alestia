//! Astronomical timekeeping (`Date`, Julian Date, `UTC`/`TAI`/`TT`/`TDB`, and NIST leap-second table)
//! matching `celastro/date.h` and `celastro/date.cpp` (`FR-008`).

use core::fmt;

use crate::constants::{DTA_SECONDS, J2000, SECONDS_PER_DAY};

/// Single leap-second entry pairing accumulated `TAI - UTC` integer seconds with its insertion JD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeapSecondRecord {
    /// Total offset `TAI - UTC` in seconds effective at `jd`.
    pub seconds: i32,
    /// UTC midnight Julian Date on which this offset takes effect.
    pub jd: f64,
}

/// Standard USNO historical leap-second insertion table (`1972-01-01` through `2017-01-01`),
/// matching Celestia's built-in `LeapSeconds` table.
pub const DEFAULT_LEAP_SECONDS: [LeapSecondRecord; 28] = [
    LeapSecondRecord {
        seconds: 10,
        jd: 2_441_317.5,
    }, // 1972-01-01
    LeapSecondRecord {
        seconds: 11,
        jd: 2_441_499.5,
    }, // 1972-07-01
    LeapSecondRecord {
        seconds: 12,
        jd: 2_441_683.5,
    }, // 1973-01-01
    LeapSecondRecord {
        seconds: 13,
        jd: 2_442_048.5,
    }, // 1974-01-01
    LeapSecondRecord {
        seconds: 14,
        jd: 2_442_413.5,
    }, // 1975-01-01
    LeapSecondRecord {
        seconds: 15,
        jd: 2_442_778.5,
    }, // 1976-01-01
    LeapSecondRecord {
        seconds: 16,
        jd: 2_443_144.5,
    }, // 1977-01-01
    LeapSecondRecord {
        seconds: 17,
        jd: 2_443_509.5,
    }, // 1978-01-01
    LeapSecondRecord {
        seconds: 18,
        jd: 2_443_874.5,
    }, // 1979-01-01
    LeapSecondRecord {
        seconds: 19,
        jd: 2_444_239.5,
    }, // 1980-01-01
    LeapSecondRecord {
        seconds: 20,
        jd: 2_444_786.5,
    }, // 1981-07-01
    LeapSecondRecord {
        seconds: 21,
        jd: 2_445_151.5,
    }, // 1982-07-01
    LeapSecondRecord {
        seconds: 22,
        jd: 2_445_516.5,
    }, // 1983-07-01
    LeapSecondRecord {
        seconds: 23,
        jd: 2_446_247.5,
    }, // 1985-07-01
    LeapSecondRecord {
        seconds: 24,
        jd: 2_447_161.5,
    }, // 1988-01-01
    LeapSecondRecord {
        seconds: 25,
        jd: 2_447_892.5,
    }, // 1990-01-01
    LeapSecondRecord {
        seconds: 26,
        jd: 2_448_257.5,
    }, // 1991-01-01
    LeapSecondRecord {
        seconds: 27,
        jd: 2_448_804.5,
    }, // 1992-07-01
    LeapSecondRecord {
        seconds: 28,
        jd: 2_449_169.5,
    }, // 1993-07-01
    LeapSecondRecord {
        seconds: 29,
        jd: 2_449_534.5,
    }, // 1994-07-01
    LeapSecondRecord {
        seconds: 30,
        jd: 2_450_083.5,
    }, // 1996-01-01
    LeapSecondRecord {
        seconds: 31,
        jd: 2_450_630.5,
    }, // 1997-07-01
    LeapSecondRecord {
        seconds: 32,
        jd: 2_451_179.5,
    }, // 1999-01-01
    LeapSecondRecord {
        seconds: 33,
        jd: 2_453_736.5,
    }, // 2006-01-01
    LeapSecondRecord {
        seconds: 34,
        jd: 2_454_832.5,
    }, // 2009-01-01
    LeapSecondRecord {
        seconds: 35,
        jd: 2_456_109.5,
    }, // 2012-07-01
    LeapSecondRecord {
        seconds: 36,
        jd: 2_457_204.5,
    }, // 2015-07-01
    LeapSecondRecord {
        seconds: 37,
        jd: 2_457_754.5,
    }, // 2017-01-01
];

/// NTP epoch (`1900-01-01T00:00:00Z`) expressed as Julian Date.
const NTP_EPOCH_JD: f64 = 2_415_020.5;

/// Errors emitted when parsing or validating a NIST `leap-seconds.list` table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeapSecondsParseError {
    /// Input contained zero leap-second records.
    EmptyTable,
    /// A non-comment line could not be parsed into `(ntp_seconds, tai_minus_utc)`.
    MalformedLine {
        /// 1-based line number of the invalid entry.
        line: usize,
    },
    /// Records were not strictly ordered by increasing Julian Date.
    NonMonotonic {
        /// 1-based line number violating monotonic ordering.
        line: usize,
    },
}

impl fmt::Display for LeapSecondsParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTable => write!(f, "leap-second table is empty"),
            Self::MalformedLine { line } => {
                write!(f, "malformed leap-second entry on line {line}")
            }
            Self::NonMonotonic { line } => {
                write!(f, "non-monotonic leap-second timestamp on line {line}")
            }
        }
    }
}

impl std::error::Error for LeapSecondsParseError {}

/// Immutable leap-second schedule supporting both the built-in USNO table and runtime-parsed
/// NIST `leap-seconds.list` files.
#[derive(Clone, Debug, PartialEq)]
pub struct LeapSecondTable {
    records: Vec<LeapSecondRecord>,
}

impl Default for LeapSecondTable {
    fn default() -> Self {
        Self::default_table()
    }
}

impl LeapSecondTable {
    /// Construct a table initialized with [`DEFAULT_LEAP_SECONDS`].
    #[must_use]
    pub fn default_table() -> Self {
        Self {
            records: DEFAULT_LEAP_SECONDS.to_vec(),
        }
    }

    /// Construct a validated table from an explicit record list.
    ///
    /// # Errors
    /// Returns [`LeapSecondsParseError::EmptyTable`] if `records` is empty or
    /// [`LeapSecondsParseError::NonMonotonic`] if Julian Dates are not strictly increasing.
    pub fn from_records(records: Vec<LeapSecondRecord>) -> Result<Self, LeapSecondsParseError> {
        if records.is_empty() {
            return Err(LeapSecondsParseError::EmptyTable);
        }
        for (idx, window) in records.windows(2).enumerate() {
            if window[1].jd <= window[0].jd {
                return Err(LeapSecondsParseError::NonMonotonic { line: idx + 2 });
            }
        }
        Ok(Self { records })
    }

    /// Parse an IETF/NIST `leap-seconds.list` file (`#` comments, NTP timestamps + offsets).
    ///
    /// # Errors
    /// Returns [`LeapSecondsParseError`] if any non-comment line is malformed, non-monotonic,
    /// or yields an empty table.
    pub fn parse_nist_list(input: &str) -> Result<Self, LeapSecondsParseError> {
        let mut records: Vec<LeapSecondRecord> = Vec::new();
        for (idx, raw_line) in input.lines().enumerate() {
            let line_num = idx + 1;
            let without_comment = raw_line.split('#').next().unwrap_or("");
            let trimmed = without_comment.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut fields = trimmed.split_whitespace();
            let Some(ntp_token) = fields.next() else {
                return Err(LeapSecondsParseError::MalformedLine { line: line_num });
            };
            let Some(offset_token) = fields.next() else {
                return Err(LeapSecondsParseError::MalformedLine { line: line_num });
            };
            let Ok(ntp_secs) = ntp_token.parse::<u64>() else {
                return Err(LeapSecondsParseError::MalformedLine { line: line_num });
            };
            let Ok(offset_secs) = offset_token.parse::<i32>() else {
                return Err(LeapSecondsParseError::MalformedLine { line: line_num });
            };
            let jd = NTP_EPOCH_JD + ntp_seconds_to_days(ntp_secs);
            if let Some(prev) = records.last().copied()
                && jd <= prev.jd
            {
                return Err(LeapSecondsParseError::NonMonotonic { line: line_num });
            }
            records.push(LeapSecondRecord {
                seconds: offset_secs,
                jd,
            });
        }
        if records.is_empty() {
            return Err(LeapSecondsParseError::EmptyTable);
        }
        Ok(Self { records })
    }

    /// Borrow all leap-second insertion records in chronological order.
    #[must_use]
    pub fn records(&self) -> &[LeapSecondRecord] {
        &self.records
    }

    /// Convert International Atomic Time (TAI Julian Date) to a UTC calendar [`Date`].
    #[must_use]
    pub fn tai_to_utc(&self, tai_jd: f64) -> Date {
        let mut d_at = self.records[0].seconds;
        let mut extra_secs = 0_i32;

        for idx in (1..self.records.len()).rev() {
            let curr = self.records[idx];
            let prev = self.records[idx - 1];
            if tai_jd - secs_to_days(f64::from(curr.seconds)) >= curr.jd {
                d_at = curr.seconds;
                break;
            }
            if tai_jd - secs_to_days(f64::from(prev.seconds)) >= curr.jd {
                d_at = curr.seconds;
                extra_secs = curr.seconds - prev.seconds;
                break;
            }
        }

        let mut utc_date = Date::from_julian_date(tai_jd - secs_to_days(f64::from(d_at)));
        utc_date.seconds += f64::from(extra_secs);
        utc_date
    }

    /// Convert a UTC calendar [`Date`] to International Atomic Time (TAI Julian Date).
    #[must_use]
    pub fn utc_to_tai(&self, utc: &Date) -> f64 {
        let mut d_at = self.records[0].seconds;
        let utc_day_jd = Date::new(utc.year, utc.month, utc.day).to_julian_date();

        for idx in (1..self.records.len()).rev() {
            if utc_day_jd >= self.records[idx].jd {
                d_at = self.records[idx].seconds;
                break;
            }
        }

        utc_day_jd
            + secs_to_days(
                f64::from(utc.hour) * 3600.0
                    + f64::from(utc.minute) * 60.0
                    + utc.seconds
                    + f64::from(d_at),
            )
    }

    /// Convert Barycentric Dynamical Time (TDB Julian Date) to a UTC calendar [`Date`].
    #[must_use]
    pub fn tdb_to_utc(&self, tdb_jd: f64) -> Date {
        self.tai_to_utc(tt_to_tai(tdb_to_tt(tdb_jd)))
    }

    /// Convert a UTC calendar [`Date`] to Barycentric Dynamical Time (TDB Julian Date).
    #[must_use]
    pub fn utc_to_tdb(&self, utc: &Date) -> f64 {
        tt_to_tdb(tai_to_tt(self.utc_to_tai(utc)))
    }

    /// Convert UTC Julian Date (ignoring intra-second leap ambiguity) to TAI Julian Date.
    #[must_use]
    pub fn jd_utc_to_tai(&self, jd_utc: f64) -> f64 {
        let mut d_at = self.records[0].seconds;
        for idx in (1..self.records.len()).rev() {
            if jd_utc > self.records[idx].jd {
                d_at = self.records[idx].seconds;
                break;
            }
        }
        jd_utc + secs_to_days(f64::from(d_at))
    }

    /// Convert TAI Julian Date to UTC Julian Date.
    #[must_use]
    pub fn tai_to_jd_utc(&self, tai_jd: f64) -> f64 {
        let mut d_at = self.records[0].seconds;
        for idx in (1..self.records.len()).rev() {
            let prev_offset = secs_to_days(f64::from(self.records[idx - 1].seconds));
            if tai_jd - prev_offset > self.records[idx].jd {
                d_at = self.records[idx].seconds;
                break;
            }
        }
        tai_jd - secs_to_days(f64::from(d_at))
    }
}

/// Calendar date and time representation matching Celestia's `celestia::astro::Date`.
#[derive(Clone, Debug, PartialEq)]
pub struct Date {
    /// Astronomical year (`0` = 1 BCE, negative values for earlier BCE years).
    pub year: i32,
    /// Month (`1..=12`).
    pub month: u8,
    /// Day of month (`1..=31`).
    pub day: u8,
    /// Hour of day (`0..=23`).
    pub hour: u8,
    /// Minute of hour (`0..=59`).
    pub minute: u8,
    /// Seconds (`0.0..61.0`, `>= 60.0` during positive leap seconds).
    pub seconds: f64,
    /// Day of week (`0` Sunday .. `6` Saturday).
    pub wday: u8,
    /// Offset from UTC in seconds.
    pub utc_offset: i32,
    /// Timezone label (default `"UTC"`).
    pub tzname: String,
}

impl Default for Date {
    fn default() -> Self {
        Self {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            seconds: 0.0,
            wday: 0,
            utc_offset: 0,
            tzname: String::from("UTC"),
        }
    }
}

impl Date {
    /// Construct a midnight date from astronomical `year`, `month` (`1..=12`), and `day` (`1..=31`).
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            ..Self::default()
        }
    }

    /// Construct a UTC calendar [`Date`] from a Julian Date, preserving the Julian -> Gregorian
    /// October 1582 transition matching Celestia `Date::Date(double jd)`.
    #[must_use]
    pub fn from_julian_date(jd: f64) -> Self {
        let day_int = floor_to_i64(jd + 0.5);
        let wday = u8::try_from((day_int + 1).rem_euclid(7)).unwrap_or(0);

        let cal_c = if day_int < 2_299_161 {
            i64_to_f64(day_int + 1524)
        } else {
            let century_b = floor_to_i64((i64_to_f64(day_int) - 1_867_216.25) / 36_524.25);
            i64_to_f64(day_int + century_b - floor_to_i64(i64_to_f64(century_b) / 4.0) + 1525)
        };

        let year_d = floor_to_i64((cal_c - 122.1) / 365.25);
        let days_e = floor_to_i64(365.25 * i64_to_f64(year_d));
        let month_f = floor_to_i64((cal_c - i64_to_f64(days_e)) / 30.6001);

        let frac_day =
            cal_c - i64_to_f64(days_e) - i64_to_f64(floor_to_i64(30.6001 * i64_to_f64(month_f)))
                + ((jd + 0.5) - i64_to_f64(day_int));

        let month_val = month_f - 1 - 12 * (month_f / 14);
        let year_val = year_d - 4715 - floor_to_i64((7.0 + i64_to_f64(month_val)) / 10.0);

        let day_num = trunc_i64(frac_day);
        let frac_hour = (frac_day - i64_to_f64(day_num)) * 24.0;
        let hour_num = trunc_i64(frac_hour);
        let frac_min = (frac_hour - i64_to_f64(hour_num)) * 60.0;
        let min_num = trunc_i64(frac_min);
        let seconds = (frac_min - i64_to_f64(min_num)) * 60.0;

        Self {
            year: clamp_to_i32(year_val),
            month: u8::try_from(month_val.clamp(0, 255)).unwrap_or(0),
            day: u8::try_from(day_num.clamp(0, 255)).unwrap_or(0),
            hour: u8::try_from(hour_num.clamp(0, 255)).unwrap_or(0),
            minute: u8::try_from(min_num.clamp(0, 255)).unwrap_or(0),
            seconds,
            wday,
            utc_offset: 0,
            tzname: String::from("UTC"),
        }
    }

    /// Convert this calendar date to a continuous Julian Date matching Celestia `Date::operator double()`.
    #[must_use]
    pub fn to_julian_date(&self) -> f64 {
        let mut adj_year = self.year;
        let mut adj_month = i32::from(self.month);
        if self.month <= 2 {
            adj_year = self.year - 1;
            adj_month += 12;
        }

        let gregorian_corr = if self.year > 1582
            || (self.year == 1582 && (self.month > 10 || (self.month == 10 && self.day >= 15)))
        {
            adj_year / 400 - adj_year / 100
        } else {
            -2
        };

        (365.25 * f64::from(adj_year)).floor()
            + (30.6001 * f64::from(adj_month + 1)).floor()
            + f64::from(gregorian_corr)
            + 1_720_996.5
            + f64::from(self.day)
            + f64::from(self.hour) / 24.0
            + f64::from(self.minute) / 1440.0
            + self.seconds / SECONDS_PER_DAY
    }

    /// Parse an ISO-8601 (`YYYY-MM-DD[T| ]HH:MM[:SS[.fff]][Z|+00:00]`) or legacy whitespace
    /// (`YYYY M D [HH:MM[:SS[.fff]]]`) date string, matching Celestia `parseDate`.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim_matches([' ', '\t']);
        if trimmed.is_empty() {
            return None;
        }

        let parsed = parse_iso(trimmed).or_else(|| parse_legacy(trimmed))?;
        if parsed.month < 1
            || parsed.month > 12
            || parsed.hour > 23
            || parsed.minute > 59
            || !parsed.seconds.is_finite()
            || parsed.seconds >= 60.0
            || parsed.seconds < 0.0
        {
            return None;
        }

        let max_day = days_in_month(parsed.year, parsed.month);
        if parsed.day < 1 || parsed.day > max_day {
            return None;
        }
        Some(parsed)
    }

    /// Format this date as an ISO-8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SS.sssssZ`).
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:08.5}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.seconds
        )
    }
}

impl From<&Date> for f64 {
    fn from(date: &Date) -> Self {
        date.to_julian_date()
    }
}

/// Convert seconds into Julian days (`s / 86400`).
#[must_use]
pub const fn secs_to_days(seconds: f64) -> f64 {
    seconds * (1.0 / SECONDS_PER_DAY)
}

/// Convert Julian days into seconds (`d * 86400`).
#[must_use]
pub const fn days_to_secs(days: f64) -> f64 {
    days * SECONDS_PER_DAY
}

/// Alias for [`secs_to_days`], matching `secondsToJulianDate`.
#[must_use]
pub const fn seconds_to_julian_date(seconds: f64) -> f64 {
    seconds / SECONDS_PER_DAY
}

/// Alias for [`days_to_secs`], matching `julianDateToSeconds`.
#[must_use]
pub const fn julian_date_to_seconds(jd: f64) -> f64 {
    jd * SECONDS_PER_DAY
}

/// Convert Terrestrial Time (TT JD) to International Atomic Time (TAI JD).
#[must_use]
pub const fn tt_to_tai(tt_jd: f64) -> f64 {
    tt_jd - secs_to_days(DTA_SECONDS)
}

/// Convert International Atomic Time (TAI JD) to Terrestrial Time (TT JD).
#[must_use]
pub const fn tai_to_tt(tai_jd: f64) -> f64 {
    tai_jd + secs_to_days(DTA_SECONDS)
}

/// Compute periodic `TDB - TT` relativistic correction in seconds for a dynamical Julian Date.
#[must_use]
pub fn tdb_correction(tdb_jd: f64) -> f64 {
    const AMP_SEC: f64 = 1.657e-3;
    const ECC: f64 = 1.671e-2;
    const M0: f64 = 6.239_996;
    const M1: f64 = 1.990_968_71e-7;

    let elapsed_secs = days_to_secs(tdb_jd - J2000);
    let mean_anom = M0 + M1 * elapsed_secs;
    let ecc_anom = mean_anom + ECC * mean_anom.sin();
    AMP_SEC * ecc_anom.sin()
}

/// Convert Terrestrial Time (TT JD) to Barycentric Dynamical Time (TDB JD).
#[must_use]
pub fn tt_to_tdb(tt_jd: f64) -> f64 {
    tt_jd + secs_to_days(tdb_correction(tt_jd))
}

/// Convert Barycentric Dynamical Time (TDB JD) to Terrestrial Time (TT JD).
#[must_use]
pub fn tdb_to_tt(tdb_jd: f64) -> f64 {
    tdb_jd - secs_to_days(tdb_correction(tdb_jd))
}

/// Convert a UTC [`Date`] to TAI Julian Date using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn utc_to_tai(utc: &Date) -> f64 {
    LeapSecondTable::default_table().utc_to_tai(utc)
}

/// Convert a TAI Julian Date to a UTC [`Date`] using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn tai_to_utc(tai_jd: f64) -> Date {
    LeapSecondTable::default_table().tai_to_utc(tai_jd)
}

/// Convert a UTC [`Date`] to TDB Julian Date using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn utc_to_tdb(utc: &Date) -> f64 {
    LeapSecondTable::default_table().utc_to_tdb(utc)
}

/// Convert a TDB Julian Date to a UTC [`Date`] using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn tdb_to_utc(tdb_jd: f64) -> Date {
    LeapSecondTable::default_table().tdb_to_utc(tdb_jd)
}

/// Convert a UTC Julian Date to TAI Julian Date using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn jd_utc_to_tai(jd_utc: f64) -> f64 {
    LeapSecondTable::default_table().jd_utc_to_tai(jd_utc)
}

/// Convert a TAI Julian Date to UTC Julian Date using [`DEFAULT_LEAP_SECONDS`].
#[must_use]
pub fn tai_to_jd_utc(tai_jd: f64) -> f64 {
    LeapSecondTable::default_table().tai_to_jd_utc(tai_jd)
}

fn days_in_month(year: i32, month: u8) -> u8 {
    if month == 2 {
        if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            29
        } else {
            28
        }
    } else if matches!(month, 4 | 6 | 9 | 11) {
        30
    } else {
        31
    }
}

fn parse_iso(input: &str) -> Option<Date> {
    let mut rest = input;
    let year = consume_signed_i32(&mut rest, None)?;
    rest = rest.strip_prefix('-')?;
    let month = consume_u8(&mut rest, Some(2))?;
    rest = rest.strip_prefix('-')?;
    let day = consume_u8(&mut rest, Some(2))?;
    if rest.is_empty() {
        return Some(Date::new(year, month, day));
    }
    let after_sep = rest.strip_prefix('T').or_else(|| rest.strip_prefix(' '))?;
    let (hour, minute, seconds) = parse_time(after_sep)?;
    let mut date = Date::new(year, month, day);
    date.hour = hour;
    date.minute = minute;
    date.seconds = seconds;
    Some(date)
}

fn parse_legacy(input: &str) -> Option<Date> {
    let mut rest = input;
    let year = consume_signed_i32(&mut rest, None)?;
    consume_ws(&mut rest)?;
    let month = consume_u8(&mut rest, None)?;
    consume_ws(&mut rest)?;
    let day = consume_u8(&mut rest, None)?;
    if rest.is_empty() {
        return Some(Date::new(year, month, day));
    }
    consume_ws(&mut rest)?;
    let (hour, minute, seconds) = parse_time(rest)?;
    let mut date = Date::new(year, month, day);
    date.hour = hour;
    date.minute = minute;
    date.seconds = seconds;
    Some(date)
}

fn parse_time(input: &str) -> Option<(u8, u8, f64)> {
    let mut rest = input;
    let hour = consume_u8(&mut rest, Some(2))?;
    rest = rest.strip_prefix(':')?;
    let minute = consume_u8(&mut rest, Some(2))?;
    if rest.is_empty() {
        return Some((hour, minute, 0.0));
    }
    rest = rest.strip_prefix(':')?;
    if rest.len() < 2 {
        return None;
    }
    let bytes = rest.as_bytes();
    if !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return None;
    }
    let mut end = 2_usize;
    if bytes.get(2) == Some(&b'.') {
        end = 3;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == 3 {
            return None;
        }
    }
    let seconds = rest[..end].parse::<f64>().ok()?;
    let suffix = &rest[end..];
    if suffix.is_empty() || suffix == "Z" || suffix == "+00:00" {
        Some((hour, minute, seconds))
    } else {
        None
    }
}

fn consume_ws(rest: &mut &str) -> Option<()> {
    let count = rest
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    if count == 0 {
        return None;
    }
    *rest = &rest[count..];
    Some(())
}

fn consume_signed_i32(rest: &mut &str, exact_len: Option<usize>) -> Option<i32> {
    let bytes = rest.as_bytes();
    let mut end = 0_usize;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || (end == 1 && !bytes[0].is_ascii_digit()) {
        return None;
    }
    if let Some(expected) = exact_len
        && end != expected
    {
        return None;
    }
    let parsed = rest[..end].parse::<i32>().ok()?;
    *rest = &rest[end..];
    Some(parsed)
}

fn consume_u8(rest: &mut &str, exact_len: Option<usize>) -> Option<u8> {
    let end = rest.bytes().take_while(u8::is_ascii_digit).count();
    if end == 0 {
        return None;
    }
    if let Some(expected) = exact_len
        && end != expected
    {
        return None;
    }
    let parsed = rest[..end].parse::<u8>().ok()?;
    *rest = &rest[end..];
    Some(parsed)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "NTP second counts around 1972-2100 fit comfortably within 53-bit f64 precision"
)]
const fn ntp_seconds_to_days(ntp_secs: u64) -> f64 {
    (ntp_secs as f64) / SECONDS_PER_DAY
}

#[expect(
    clippy::cast_precision_loss,
    reason = "Julian day counters within astronomical epochs fit comfortably within 53-bit f64 precision"
)]
const fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Julian day arithmetic outputs are clamped within finite calendar bounds before truncation"
)]
fn floor_to_i64(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    value.floor().clamp(-1e15, 1e15) as i64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Calendar component counts are bounded in 0..366 before truncation"
)]
fn trunc_i64(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    value.trunc().clamp(-1e15, 1e15) as i64
}

fn clamp_to_i32(value: i64) -> i32 {
    i32::try_from(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LEAP_SECONDS, Date, LeapSecondRecord, LeapSecondTable, LeapSecondsParseError,
        days_to_secs, jd_utc_to_tai, julian_date_to_seconds, seconds_to_julian_date, secs_to_days,
        tai_to_jd_utc, tai_to_tt, tai_to_utc, tdb_to_tt, tdb_to_utc, tt_to_tai, tt_to_tdb,
        utc_to_tai, utc_to_tdb,
    };
    use crate::constants::J2000;

    #[test]
    fn date_parsing_accepts_all_upstream_celestia_cases() -> Result<(), String> {
        let cases = [
            ("2026-10-24", 2026, 10, 24, 0, 0, 0.0),
            ("2026-10-24T15:04:31.5", 2026, 10, 24, 15, 4, 31.5),
            ("2026-10-24T15:04:31.5Z", 2026, 10, 24, 15, 4, 31.5),
            ("2026-10-24T15:04:31.5+00:00", 2026, 10, 24, 15, 4, 31.5),
            ("  2026-10-24T09:35:21.25", 2026, 10, 24, 9, 35, 21.25),
            ("2000\t02\t29", 2000, 2, 29, 0, 0, 0.0),
            ("-350\t5\t15 11:15", -350, 5, 15, 11, 15, 0.0),
            ("1900\t2 28 15:20:04.75", 1900, 2, 28, 15, 20, 4.75),
        ];

        for (raw, year, month, day, hour, minute, seconds) in cases {
            let Some(parsed) = Date::parse(raw) else {
                return Err(format!("Expected date {raw:?} to parse"));
            };
            assert_eq!(parsed.year, year);
            assert_eq!(parsed.month, month);
            assert_eq!(parsed.day, day);
            assert_eq!(parsed.hour, hour);
            assert_eq!(parsed.minute, minute);
            if (parsed.seconds - seconds).abs() > 1e-12 {
                return Err(format!("Seconds mismatch for {raw:?}: {}", parsed.seconds));
            }
        }
        Ok(())
    }

    #[test]
    fn date_parsing_rejects_all_upstream_failure_cases() {
        for invalid in [
            "2026-13-05",
            "2026-13-05T15:04:31.5",
            "2026-09-15T36:05:11.2",
            "1900-02-29T12:00:00",
            "1900 02 29 12:00:00",
            "1900 2 28 xkjlyz",
            "",
            "2026-01-01T12:00:00.",
        ] {
            assert_eq!(
                Date::parse(invalid),
                None,
                "Expected rejection for {invalid:?}"
            );
        }
    }

    #[test]
    fn julian_date_and_timescales_roundtrip_accurately() -> Result<(), String> {
        let mut j2000_noon = Date::new(2000, 1, 1);
        j2000_noon.hour = 12;
        if (f64::from(&j2000_noon) - J2000).abs() > 1e-12 {
            return Err("J2000 epoch JD mismatch".to_owned());
        }
        let round = Date::from_julian_date(J2000);
        if round.year != 2000 || round.month != 1 || round.day != 1 || round.hour != 12 {
            return Err(format!("J2000 reverse conversion failed: {round:?}"));
        }
        if !round.to_iso8601().starts_with("2000-01-01T12:00:") {
            return Err(format!("Unexpected ISO8601 output: {}", round.to_iso8601()));
        }

        // Pre-1582 Julian date check
        let julian_past = Date::new(-350, 5, 15);
        let past_back = Date::from_julian_date(julian_past.to_julian_date());
        assert_eq!(
            (past_back.year, past_back.month, past_back.day),
            (-350, 5, 15)
        );

        let utc_sample =
            Date::parse("2025-06-15T18:30:12.5").ok_or_else(|| "Sample parse failed".to_owned())?;
        let tdb = utc_to_tdb(&utc_sample);
        let back = tdb_to_utc(tdb);
        if back.year != 2025
            || back.month != 6
            || back.day != 15
            || back.hour != 18
            || back.minute != 30
            || (back.seconds - 12.5).abs() > 1e-4
        {
            return Err(format!("UTC<->TDB roundtrip drifted: {back:?}"));
        }

        let tai_val = utc_to_tai(&utc_sample);
        let utc_back = tai_to_utc(tai_val);
        if (utc_back.seconds - 12.5).abs() > 1e-4 {
            return Err("UTC<->TAI seconds drifted".to_owned());
        }
        if (tai_to_tt(tt_to_tai(J2000)) - J2000).abs() > 1e-14 {
            return Err("TT<->TAI roundtrip failed".to_owned());
        }
        if (tdb_to_tt(tt_to_tdb(J2000)) - J2000).abs() > 1e-12 {
            return Err("TT<->TDB roundtrip failed".to_owned());
        }
        if (tai_to_jd_utc(jd_utc_to_tai(J2000)) - J2000).abs() > 1e-12 {
            return Err("JDUTC<->TAI roundtrip failed".to_owned());
        }
        if (days_to_secs(secs_to_days(42.0)) - 42.0).abs() > 1e-12
            || (julian_date_to_seconds(seconds_to_julian_date(42.0)) - 42.0).abs() > 1e-12
        {
            return Err("Day/second aliases drifted".to_owned());
        }
        Ok(())
    }

    #[test]
    fn nist_leap_seconds_list_parses_and_validates_errors() -> Result<(), String> {
        let sample = "\
# NIST leap-seconds.list excerpt
#$ 3676924800
2272060800\t10\t# 1 Jan 1972
2287785600\t11\t# 1 Jul 1972
3692217600\t37\t# 1 Jan 2017
";
        let table = LeapSecondTable::parse_nist_list(sample)
            .map_err(|err| format!("Failed to parse sample NIST list: {err}"))?;
        assert_eq!(table.records().len(), 3);
        assert_eq!(table.records()[0], DEFAULT_LEAP_SECONDS[0]);
        assert_eq!(table.records()[2], DEFAULT_LEAP_SECONDS[27]);

        assert_eq!(
            LeapSecondTable::parse_nist_list("# comments only\n"),
            Err(LeapSecondsParseError::EmptyTable)
        );
        assert_eq!(
            LeapSecondTable::parse_nist_list("not_a_number 37"),
            Err(LeapSecondsParseError::MalformedLine { line: 1 })
        );
        assert_eq!(
            LeapSecondTable::from_records(vec![
                LeapSecondRecord {
                    seconds: 11,
                    jd: 2_441_500.0
                },
                LeapSecondRecord {
                    seconds: 10,
                    jd: 2_441_400.0
                },
            ]),
            Err(LeapSecondsParseError::NonMonotonic { line: 2 })
        );
        Ok(())
    }
}
