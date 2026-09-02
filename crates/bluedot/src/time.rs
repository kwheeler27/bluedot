//! Minimal calendar types: a `Date` and a UTC `Timestamp`.
//!
//! The fact schema needs dates as `YYYY-MM-DD` strings and a UTC timestamp for
//! `retrieved_at`. A date crate would do this, but the conversion from "seconds
//! since 1970" to a civil date is ~15 lines of integer arithmetic, so we keep the
//! dependency list short and the math visible.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Serialize, Serializer};

/// A calendar date (proleptic Gregorian). `Copy` because it is three small integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Date { year, month, day }
    }

    /// Date for a count of days since 1970-01-01 (negative = before).
    ///
    /// Howard Hinnant's `civil_from_days` algorithm: it shifts the epoch to
    /// 0000-03-01 so leap days land at the end of the year, works in 400-year
    /// "eras" (146097 days each), and undoes the shift at the end.
    pub fn from_unix_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // day of era        [0, 146096]
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // year of era [0, 399]
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year (March-based) [0, 365]
        let mp = (5 * doy + 2) / 153; // month index, March = 0  [0, 11]
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Date {
            year: year as i32,
            month,
            day,
        }
    }
}

impl Date {
    /// The following calendar day — for the one-day half-open interval of a
    /// point-in-time observation (ADR-0013). Month/year rollover handled;
    /// leap years by the divisibility rule.
    pub fn next_day(self) -> Date {
        let leap = self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0);
        let dim = match self.month {
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        match (self.day < dim, self.month < 12) {
            (true, _) => Date::new(self.year, self.month, self.day + 1),
            (false, true) => Date::new(self.year, self.month + 1, 1),
            (false, false) => Date::new(self.year + 1, 1, 1),
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

// Serialize as the ISO string, not as a `{year, month, day}` object.
// `collect_str` serializes anything that implements `Display`.
impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A UTC instant with second precision, serialized as RFC 3339 (`2026-08-31T04:05:06Z`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub unix_seconds: i64,
}

impl Timestamp {
    pub fn now() -> Self {
        // `duration_since` fails only if the clock is before 1970; treat that as 0.
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Timestamp { unix_seconds: secs }
    }

    pub fn date(&self) -> Date {
        // `div_euclid` rounds toward negative infinity, so pre-1970 instants map
        // to the right day (plain `/` truncates toward zero).
        Date::from_unix_days(self.unix_seconds.div_euclid(86_400))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sod = self.unix_seconds.rem_euclid(86_400); // seconds into the day
        write!(
            f,
            "{}T{:02}:{:02}:{:02}Z",
            self.date(),
            sod / 3_600,
            (sod % 3_600) / 60,
            sod % 60
        )
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970_01_01() {
        assert_eq!(Date::from_unix_days(0), Date::new(1970, 1, 1));
    }

    #[test]
    fn leap_day_2000() {
        // 30 years × 365 + 7 leap days (72..96) = 10957 days to 2000-01-01; +59 to Feb 29.
        assert_eq!(Date::from_unix_days(10_957 + 59), Date::new(2000, 2, 29));
    }

    #[test]
    fn day_before_epoch() {
        assert_eq!(Date::from_unix_days(-1), Date::new(1969, 12, 31));
    }

    #[test]
    fn next_day_rolls_months_years_and_leap_days() {
        assert_eq!(Date::new(2026, 9, 2).next_day(), Date::new(2026, 9, 3));
        assert_eq!(Date::new(2026, 9, 30).next_day(), Date::new(2026, 10, 1));
        assert_eq!(Date::new(2026, 12, 31).next_day(), Date::new(2027, 1, 1));
        assert_eq!(Date::new(2024, 2, 28).next_day(), Date::new(2024, 2, 29));
        assert_eq!(Date::new(2023, 2, 28).next_day(), Date::new(2023, 3, 1));
        assert_eq!(Date::new(2000, 2, 28).next_day(), Date::new(2000, 2, 29));
        assert_eq!(Date::new(1900, 2, 28).next_day(), Date::new(1900, 3, 1));
    }

    #[test]
    fn timestamp_formats_rfc3339() {
        let t = Timestamp {
            unix_seconds: 1_700_000_000,
        };
        assert_eq!(t.to_string(), "2023-11-14T22:13:20Z");
        assert_eq!(
            serde_json::to_string(&t).unwrap(),
            "\"2023-11-14T22:13:20Z\""
        );
        assert_eq!(
            serde_json::to_string(&Date::new(2024, 12, 12)).unwrap(),
            "\"2024-12-12\""
        );
    }
}
