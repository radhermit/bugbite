use std::fmt;
use std::ops::RangeBounds;
use std::str::FromStr;
use std::sync::LazyLock;

use base64::prelude::*;
use regex::Regex;
use serde::Serialize;
use serde_with::{DeserializeFromStr, SerializeDisplay, skip_serializing_none};

use crate::Error;
use crate::traits::Contains;

pub mod bugzilla;
pub mod github;
pub mod redmine;

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

impl<T: Eq> RangeOrValue<T> {
    pub fn matches<V>(&self, item: &V) -> bool
    where
        T: PartialEq<V>,
        T: PartialOrd<V>,
        V: PartialOrd<T>,
    {
        match self {
            Self::Value(value) => value == item,
            Self::RangeOp(value) => value.matches(item),
            Self::Range(value) => value.matches(item),
        }
    }
}

impl<T> FromStr for RangeOrValue<T>
where
    T: FromStr + Eq,
    T::Err: std::fmt::Display + std::fmt::Debug,
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

impl<T: Eq> RangeOp<T> {
    pub fn matches<V>(&self, item: &V) -> bool
    where
        T: PartialEq<V>,
        T: PartialOrd<V>,
        V: PartialOrd<T>,
    {
        match self {
            Self::Less(value) => item < value,
            Self::LessOrEqual(value) => item <= value,
            Self::Equal(value) => item == value,
            Self::NotEqual(value) => item != value,
            Self::GreaterOrEqual(value) => item >= value,
            Self::Greater(value) => item > value,
        }
    }
}

impl<T> FromStr for RangeOp<T>
where
    T: FromStr + Eq,
    T::Err: std::fmt::Display + std::fmt::Debug,
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

impl<T: Eq> Range<T> {
    pub fn matches<V>(&self, item: &V) -> bool
    where
        T: PartialEq<V>,
        T: PartialOrd<V>,
        V: PartialOrd<T>,
    {
        match self {
            Self::Range(r) => r.contains(item),
            Self::Inclusive(r) => r.contains(item),
            Self::To(r) => r.contains(item),
            Self::ToInclusive(r) => r.contains(item),
            Self::From(r) => r.contains(item),
            Self::Full(r) => r.contains(item),
        }
    }
}

impl<T> FromStr for Range<T>
where
    T: FromStr + Eq,
    T::Err: std::fmt::Display + std::fmt::Debug,
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

/// Supported change variants for set-based fields.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Eq, PartialEq, Clone)]
pub enum SetChange<T> {
    Add(T),
    Remove(T),
    Set(T),
}

impl<T: FromStr> FromStr for SetChange<T> {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Some(value) = s.strip_prefix('+') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Add(value))
        } else if let Some(value) = s.strip_prefix('-') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Remove(value))
        } else {
            let value = s
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Set(value))
        }
    }
}

impl<T: fmt::Display> fmt::Display for SetChange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Add(value) => write!(f, "+{value}"),
            Self::Remove(value) => write!(f, "-{value}"),
            Self::Set(value) => value.fmt(f),
        }
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct SetChanges<T> {
    pub add: Option<Vec<T>>,
    pub remove: Option<Vec<T>>,
    pub set: Option<Vec<T>>,
}

impl<'a, T: FromStr> FromIterator<&'a SetChange<T>> for SetChanges<&'a T> {
    fn from_iter<I: IntoIterator<Item = &'a SetChange<T>>>(iterable: I) -> Self {
        let (mut add, mut remove, mut set) = (vec![], vec![], vec![]);
        for change in iterable {
            match change {
                SetChange::Add(value) => add.push(value),
                SetChange::Remove(value) => remove.push(value),
                SetChange::Set(value) => set.push(value),
            }
        }

        let set = if !set.is_empty() || (add.is_empty() && remove.is_empty()) {
            Some(set)
        } else {
            None
        };

        Self {
            add: Some(add),
            remove: Some(remove),
            set,
        }
    }
}
