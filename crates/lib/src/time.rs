use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, offset::Utc};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;
use crate::traits::Api;

mod delta;
pub use delta::TimeDelta;
mod r#static;
pub use r#static::TimeStatic;

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone, PartialEq, Eq)]
pub enum TimeDeltaOrStatic {
    Delta(TimeDelta),
    Static(TimeStatic),
}

impl PartialOrd<DateTime<Utc>> for TimeDeltaOrStatic {
    fn partial_cmp(&self, other: &DateTime<Utc>) -> Option<Ordering> {
        match self {
            Self::Delta(value) => value.partial_cmp(other),
            Self::Static(value) => value.partial_cmp(other),
        }
    }
}

impl PartialOrd<TimeDeltaOrStatic> for DateTime<Utc> {
    fn partial_cmp(&self, other: &TimeDeltaOrStatic) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}

impl PartialEq<DateTime<Utc>> for TimeDeltaOrStatic {
    fn eq(&self, other: &DateTime<Utc>) -> bool {
        match self {
            Self::Delta(value) => value < other,
            Self::Static(value) => value.eq(other),
        }
    }
}

impl PartialEq<TimeDeltaOrStatic> for DateTime<Utc> {
    fn eq(&self, other: &TimeDeltaOrStatic) -> bool {
        other.eq(self)
    }
}

impl FromStr for TimeDeltaOrStatic {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Ok(value) = s.parse() {
            Ok(Self::Delta(value))
        } else if let Ok(value) = s.parse() {
            Ok(Self::Static(value))
        } else {
            Err(Error::InvalidValue(format!("invalid time: {s}")))
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

#[cfg(test)]
mod tests {
    use crate::test::assert_err_re;

    use super::*;

    #[test]
    fn parse() {
        // invalid
        for s in ["", "0", "1", "-1", "01:02:03"] {
            let err = TimeDeltaOrStatic::from_str(s).unwrap_err();
            assert_err_re!(err, format!("invalid time: {s}"));
        }

        // valid
        for s in ["0000", "0001", "2020", "2020-08-09", "1y", "2m", "3d"] {
            let time = TimeDeltaOrStatic::from_str(s).unwrap();
            assert_eq!(time.to_string(), s);
            assert_eq!(time.as_ref(), s);
            let api = time.api();
            assert!(DateTime::parse_from_rfc3339(&api).is_ok());
        }
    }
}
