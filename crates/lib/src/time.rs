use std::fmt;
use std::str::FromStr;

use chronoutil::RelativeDuration;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

static RELATIVE_TIME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?<value>\d+)(?<unit>[ymwdhs]|min)$").unwrap());

/// Supported service variants
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub struct TimeDelta {
    raw: String,
    delta: RelativeDuration,
}

impl TimeDelta {
    pub fn delta(&self) -> RelativeDuration {
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
                x => panic!("invalid time interval unit: {x}"),
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
