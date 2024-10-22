use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use chrono::offset::Utc;
use chronoutil::RelativeDuration;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

static RELATIVE_TIME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<value>\d+)(?<unit>[[:alpha:]]+)$").unwrap());

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
                _ => return Err(Error::InvalidValue(format!("invalid time unit: {unit}"))),
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

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use crate::test::assert_err_re;

    use super::*;

    #[test]
    fn parse() {
        // invalid
        for s in ["", "1", "2h2"] {
            let err = TimeDelta::from_str(s).unwrap_err();
            assert_err_re!(err, format!("invalid time interval: {s}"));
        }

        // invalid unit
        for unit in ["z", "seconds", "ms"] {
            let s = format!("1{unit}");
            let err = TimeDelta::from_str(&s).unwrap_err();
            assert_err_re!(err, format!("invalid time unit: {unit}"));
        }

        // i32 overflow
        for unit in ["y", "m"] {
            let value = "2147483648";
            let s = format!("{value}{unit}");
            let err = TimeDelta::from_str(&s).unwrap_err();
            assert_err_re!(err, format!("invalid time interval value: {value}"));
        }

        // i64 overflow
        for unit in ["w", "d", "h", "min", "s"] {
            let value = "9223372036854775808";
            let s = format!("{value}{unit}");
            let err = TimeDelta::from_str(&s).unwrap_err();
            assert_err_re!(err, format!("invalid time interval value: {value}"));
        }

        // valid
        for s in ["1y", "2m", "3w", "4d", "5h", "10min", "100s"] {
            let delta: TimeDelta = s.try_into().unwrap();
            assert_eq!(delta.to_string(), s);
            assert_eq!(delta.as_ref(), s);

            // verify Api trait produces valid RFC3339 values
            let api = delta.api();
            assert!(DateTime::parse_from_rfc3339(&api).is_ok());
        }
    }
}
