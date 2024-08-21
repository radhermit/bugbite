use std::fmt;
use std::str::FromStr;

use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

mod delta;
pub use delta::TimeDelta;
mod r#static;
pub use r#static::TimeStatic;

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
