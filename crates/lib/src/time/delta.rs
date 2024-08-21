use std::fmt;
use std::str::FromStr;

use chrono::offset::Utc;
use chronoutil::RelativeDuration;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

static RELATIVE_TIME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<value>\d+)(?<unit>[ymwdhs]|min)$").unwrap());

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

/// Convert a raw, relative time interval to its numeric equivalent.
macro_rules! convert {
    ($s:expr) => {
        $s.parse()
            .map_err(|_| Error::InvalidValue(format!("invalid time interval value: {}", $s)))
    };
}

impl FromStr for TimeDelta {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let mut delta = RelativeDuration::zero();
        for (_, [value, unit]) in RELATIVE_TIME_RE.captures_iter(s).map(|c| c.extract()) {
            match unit {
                "y" => delta = delta + RelativeDuration::years(convert!(value)?),
                "m" => delta = delta + RelativeDuration::months(convert!(value)?),
                "w" => delta = delta + RelativeDuration::weeks(convert!(value)?),
                "d" => delta = delta + RelativeDuration::days(convert!(value)?),
                "h" => delta = delta + RelativeDuration::hours(convert!(value)?),
                "min" => delta = delta + RelativeDuration::minutes(convert!(value)?),
                "s" => delta = delta + RelativeDuration::seconds(convert!(value)?),
                _ => panic!("invalid time interval unit: {unit}"),
            }
        }

        if delta == RelativeDuration::zero() {
            Err(Error::InvalidValue(format!("invalid time interval: {s}")))
        } else {
            Ok(Self {
                raw: s.to_string(),
                delta,
            })
        }
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
