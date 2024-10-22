use std::fmt;
use std::ops::RangeBounds;
use std::str::FromStr;
use std::sync::LazyLock;

use base64::prelude::*;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Contains;
use crate::Error;

pub mod bugzilla;
pub mod github;
pub mod redmine;

/// Generic bug, issue, or ticket object.
#[derive(Debug, Eq, PartialEq)]
pub enum Item {
    Bugzilla(Box<bugzilla::Bug>),
    Github(Box<github::Issue>),
    Redmine(Box<redmine::Issue>),
}

/// Raw binary data encoded as Base64.
#[derive(DeserializeFromStr, SerializeDisplay, Default, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Base64(pub(crate) Vec<u8>);

impl FromStr for Base64 {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let data = BASE64_STANDARD
            .decode(s)
            .map_err(|e| Error::InvalidValue(format!("failed decoding base64 data: {e}")))?;
        Ok(Self(data))
    }
}

impl fmt::Display for Base64 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64_STANDARD.encode(&self.0))
    }
}

impl AsRef<[u8]> for Base64 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Base64 {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

macro_rules! stringify {
    ($field:expr) => {
        if let Some(value) = $field.as_ref() {
            value.to_string()
        } else {
            "None".to_string()
        }
    };
}
use stringify;

#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
pub enum RangeOrValue<T: Eq> {
    Value(T),
    RangeOp(RangeOp<T>),
    Range(Range<T>),
}

impl<T> FromStr for RangeOrValue<T>
where
    T: FromStr + Eq,
    <T as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse() {
            Ok(RangeOrValue::Value(value))
        } else if let Ok(value) = s.parse() {
            Ok(RangeOrValue::RangeOp(value))
        } else if let Ok(value) = s.parse() {
            Ok(RangeOrValue::Range(value))
        } else {
            Err(Error::InvalidValue(format!("invalid range or value: {s}")))
        }
    }
}

impl<T: fmt::Display + Eq> fmt::Display for RangeOrValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Value(value) => value.fmt(f),
            Self::RangeOp(value) => value.fmt(f),
            Self::Range(value) => value.fmt(f),
        }
    }
}

impl From<i64> for RangeOrValue<i64> {
    fn from(value: i64) -> Self {
        Self::Value(value)
    }
}

impl From<u64> for RangeOrValue<u64> {
    fn from(value: u64) -> Self {
        Self::Value(value)
    }
}

impl<T: Eq> From<std::ops::Range<T>> for RangeOrValue<T> {
    fn from(value: std::ops::Range<T>) -> Self {
        Self::Range(Range::Range(value))
    }
}

impl<T: Eq> From<std::ops::RangeInclusive<T>> for RangeOrValue<T> {
    fn from(value: std::ops::RangeInclusive<T>) -> Self {
        Self::Range(Range::Inclusive(value))
    }
}

impl<T: Eq> From<std::ops::RangeTo<T>> for RangeOrValue<T> {
    fn from(value: std::ops::RangeTo<T>) -> Self {
        Self::Range(Range::To(value))
    }
}

impl<T: Eq> From<std::ops::RangeToInclusive<T>> for RangeOrValue<T> {
    fn from(value: std::ops::RangeToInclusive<T>) -> Self {
        Self::Range(Range::ToInclusive(value))
    }
}

impl<T: Eq> From<std::ops::RangeFrom<T>> for RangeOrValue<T> {
    fn from(value: std::ops::RangeFrom<T>) -> Self {
        Self::Range(Range::From(value))
    }
}

impl<T: Eq> From<std::ops::RangeFull> for RangeOrValue<T> {
    fn from(value: std::ops::RangeFull) -> Self {
        Self::Range(Range::Full(value))
    }
}

static RANGE_OP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?<op>[<>]=?|!?=)(?<value>.+)$").unwrap());

#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
pub enum RangeOp<T: Eq> {
    Less(T),
    LessOrEqual(T),
    Equal(T),
    NotEqual(T),
    GreaterOrEqual(T),
    Greater(T),
}

impl<T> FromStr for RangeOp<T>
where
    T: FromStr + Eq,
    <T as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = RANGE_OP_RE.captures(s) {
            let op = caps.name("op").map_or("", |m| m.as_str());
            let value = caps.name("value").map_or("", |m| m.as_str());
            let value = value
                .parse()
                .map_err(|e| Error::InvalidValue(format!("invalid range value: {value}: {e}")))?;
            match op {
                "<" => Ok(Self::Less(value)),
                "<=" => Ok(Self::LessOrEqual(value)),
                "=" => Ok(Self::Equal(value)),
                "!=" => Ok(Self::NotEqual(value)),
                ">=" => Ok(Self::GreaterOrEqual(value)),
                ">" => Ok(Self::Greater(value)),
                _ => panic!("invalid RangeOp regex"),
            }
        } else {
            Err(Error::InvalidValue(format!("invalid range op: {s}")))
        }
    }
}

impl<T: fmt::Display + Eq> fmt::Display for RangeOp<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Less(value) => write!(f, "<{value}"),
            Self::LessOrEqual(value) => write!(f, "<={value}"),
            Self::Equal(value) => write!(f, "={value}"),
            Self::NotEqual(value) => write!(f, "!={value}"),
            Self::GreaterOrEqual(value) => write!(f, ">={value}"),
            Self::Greater(value) => write!(f, ">{value}"),
        }
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
pub enum Range<T: Eq> {
    Range(std::ops::Range<T>),                  // 0..1
    Inclusive(std::ops::RangeInclusive<T>),     // 0..=1
    To(std::ops::RangeTo<T>),                   // ..1
    ToInclusive(std::ops::RangeToInclusive<T>), // ..=1
    From(std::ops::RangeFrom<T>),               // 0..
    Full(std::ops::RangeFull),                  // ..
}

impl<T> FromStr for Range<T>
where
    T: FromStr + Eq,
    <T as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: rework to use parser combinator (winnow)
        let (start, op, finish) = if let Some((start, finish)) = s.split_once("..=") {
            (start, "..=", finish)
        } else if let Some((start, finish)) = s.split_once("..") {
            (start, "..", finish)
        } else {
            return Err(Error::InvalidValue(format!("invalid range: {s}")));
        };

        let parse = |value: &str| -> crate::Result<Option<T>> {
            if !value.is_empty() {
                Ok(Some(value.parse().map_err(|e| {
                    Error::InvalidValue(format!("invalid range value: {value}: {e}"))
                })?))
            } else {
                Ok(None)
            }
        };

        match (parse(start)?, op, parse(finish)?) {
            (Some(start), "..", Some(finish)) => {
                if start != finish {
                    Ok(Range::Range(start..finish))
                } else {
                    Err(Error::InvalidValue(format!("empty range: {s}")))
                }
            }
            (Some(start), "..", None) => Ok(Range::From(start..)),
            (None, "..", Some(finish)) => Ok(Range::To(..finish)),
            (None, "..", None) => Ok(Range::Full(..)),
            (Some(start), "..=", Some(finish)) => Ok(Range::Inclusive(start..=finish)),
            (None, "..=", Some(finish)) => Ok(Range::ToInclusive(..=finish)),
            _ => Err(Error::InvalidValue(format!("invalid range: {s}"))),
        }
    }
}

impl<T: fmt::Display + Eq> fmt::Display for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Range(r) => write!(f, "{}..{}", r.start, r.end),
            Self::Inclusive(r) => write!(f, "{}..={}", r.start(), r.end()),
            Self::To(r) => write!(f, "..{}", r.end),
            Self::ToInclusive(r) => write!(f, "..={}", r.end),
            Self::From(r) => write!(f, "{}..", r.start),
            Self::Full(_) => write!(f, ".."),
        }
    }
}

/// Return true if a type contains a given object, otherwise false.
impl<T: PartialOrd + Eq> Contains<T> for Range<T> {
    fn contains(&self, obj: &T) -> bool {
        match self {
            Self::Range(r) => r.contains(obj),
            Self::Inclusive(r) => r.contains(obj),
            Self::To(r) => r.contains(obj),
            Self::ToInclusive(r) => r.contains(obj),
            Self::From(r) => r.contains(obj),
            Self::Full(r) => r.contains(obj),
        }
    }
}
