use std::fmt;
use std::str::FromStr;

use chrono::{offset::Utc, DateTime, NaiveDate, NaiveTime};
use chronoutil::RelativeDuration;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

static STATIC_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<year>\d\d\d\d)(-(?<month>\d\d))?(-(?<day>\d\d))?$").unwrap());
static RELATIVE_TIME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<value>\d+)(?<unit>[ymwdhs]|min)$").unwrap());

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone, PartialEq, Eq)]
pub struct TimeStatic {
    raw: String,
    value: DateTime<Utc>,
}

impl FromStr for TimeStatic {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let value = if let Some(captures) = STATIC_DATE_RE.captures(s) {
            let year = captures.name("year").map(|m| m.as_str()).unwrap();
            let year = year
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid year: {year}: {e}")))?;
            let month = captures.name("month").map_or("1", |m| m.as_str());
            let month = month
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid month: {month}: {e}")))?;
            let day = captures.name("day").map_or("1", |m| m.as_str());
            let day = day
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid day: {day}: {e}")))?;
            let date = NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| Error::InvalidValue(format!("invalid static date: {s}")))?;
            let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            let dt = date.and_time(time);
            dt.and_utc()
        } else {
            DateTime::from_str(s).map_err(|e| {
                Error::InvalidValue(format!("invalid static datetime format: {s}: {e}"))
            })?
        };

        Ok(Self {
            raw: s.to_string(),
            value,
        })
    }
}

impl fmt::Display for TimeStatic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl AsRef<str> for TimeStatic {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl Api for TimeStatic {
    fn api(&self) -> String {
        self.value.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone, PartialEq, Eq)]
pub struct TimeDelta {
    raw: String,
    delta: RelativeDuration,
}

impl TimeDelta {
    fn delta(&self) -> RelativeDuration {
        self.delta
    }
}

impl FromStr for TimeDelta {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let captures: Vec<_> = RELATIVE_TIME_RE.captures_iter(s).collect();
        if captures.is_empty() {
            return Err(Error::InvalidValue(format!("invalid time interval: {s}")));
        }

        let mut delta = RelativeDuration::zero();
        for cap in captures {
            let unit = cap.name("unit").map_or("", |m| m.as_str());
            let value = cap.name("value").map_or("", |m| m.as_str());
            let value_i32: i32 = value.parse().map_err(|_| {
                Error::InvalidValue(format!("invalid time interval value: {value}"))
            })?;
            let value_i64: i64 = value.parse().map_err(|_| {
                Error::InvalidValue(format!("invalid time interval value: {value}"))
            })?;
            match unit {
                "y" => delta = delta + RelativeDuration::years(value_i32),
                "m" => delta = delta + RelativeDuration::months(value_i32),
                "w" => delta = delta + RelativeDuration::weeks(value_i64),
                "d" => delta = delta + RelativeDuration::days(value_i64),
                "h" => delta = delta + RelativeDuration::hours(value_i64),
                "min" => delta = delta + RelativeDuration::minutes(value_i64),
                "s" => delta = delta + RelativeDuration::seconds(value_i64),
                _ => panic!("invalid time interval unit: {unit}"),
            }
        }

        Ok(Self {
            raw: s.to_string(),
            delta,
        })
    }
}

impl TryFrom<&str> for TimeDelta {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl fmt::Display for TimeDelta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl AsRef<str> for TimeDelta {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl Api for TimeDelta {
    fn api(&self) -> String {
        let datetime = Utc::now() - self.delta();
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone, PartialEq, Eq)]
pub enum TimeDeltaOrStatic {
    Delta(TimeDelta),
    Static(TimeStatic),
}

impl FromStr for TimeDeltaOrStatic {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Ok(value) = s.parse() {
            Ok(Self::Delta(value))
        } else {
            Ok(Self::Static(s.parse()?))
        }
    }
}

impl fmt::Display for TimeDeltaOrStatic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Delta(value) => value.fmt(f),
            Self::Static(value) => value.fmt(f),
        }
    }
}

impl AsRef<str> for TimeDeltaOrStatic {
    fn as_ref(&self) -> &str {
        match self {
            Self::Delta(value) => value.as_ref(),
            Self::Static(value) => value.as_ref(),
        }
    }
}

impl Api for TimeDeltaOrStatic {
    fn api(&self) -> String {
        match self {
            Self::Delta(value) => value.api(),
            Self::Static(value) => value.api(),
        }
    }
}
