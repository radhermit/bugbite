use std::fmt;
use std::ops::RangeBounds;
use std::str::FromStr;

use base64::prelude::*;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Contains;
use crate::Error;

pub mod bugzilla;
pub mod github;
pub mod redmine;

/// ID type used in requests.
///
/// Some request variants support different types of IDs relating to either pulling all objects
/// from an item (e.g. requesting all attachments for a given bug ID), or the exact object ID (e.g.
/// requesting an attachment from its exact ID).
#[derive(Debug)]
pub(crate) enum Ids {
    Item(Vec<String>),
    Object(Vec<String>),
}

impl Default for Ids {
    fn default() -> Self {
        Self::Item(Default::default())
    }
}

impl Ids {
    pub(crate) fn item<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Ids::Item(ids.into_iter().map(|s| s.to_string()).collect())
    }

    pub(crate) fn object<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Ids::Object(ids.into_iter().map(|s| s.to_string()).collect())
    }

    pub(crate) fn as_slice(&self) -> IdsSlice {
        match self {
            Self::Item(ids) => IdsSlice::Item(ids.as_slice()),
            Self::Object(ids) => IdsSlice::Object(ids.as_slice()),
        }
    }
}

#[derive(Debug)]
pub(crate) enum IdsSlice<'a> {
    Item(&'a [String]),
    Object(&'a [String]),
}

/// Generic bug, issue, or ticket object.
#[derive(Debug, Eq, PartialEq)]
pub enum Item {
    Bugzilla(Box<bugzilla::Bug>),
    Github(Box<github::Issue>),
    Redmine(Box<redmine::Issue>),
}

/// Raw binary data encoded as Base64.
#[derive(DeserializeFromStr, SerializeDisplay, Default, Debug, Eq, PartialEq)]
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

#[derive(Debug, Clone)]
pub enum RangeOrEqual<T> {
    Equal(T),
    Range(Range<T>),
}

impl<T> FromStr for RangeOrEqual<T>
where
    T: FromStr + Eq,
    <T as FromStr>::Err: std::fmt::Display + std::fmt::Debug,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse() {
            Ok(RangeOrEqual::Equal(value))
        } else {
            Ok(RangeOrEqual::Range(s.parse()?))
        }
    }
}

// TODO: replace regex-based parsing with parser combinator (winnow)
static RANGE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)?(..=?)(\d+)?").unwrap());

#[derive(Debug, Clone)]
pub enum Range<T> {
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
        let Some(caps) = RANGE_RE.captures(s) else {
            return Err(Error::InvalidValue(format!("invalid range: {s}")));
        };

        let (start, op, finish) = caps.iter().skip(1).collect_tuple().unwrap();
        let start = start.map(|x| x.as_str().parse().unwrap());
        let op = op.unwrap().as_str();
        let finish = finish.map(|x| x.as_str().parse().unwrap());
        match (start, op, finish) {
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

/// Return true if a type contains a given object, otherwise false.
impl<T: PartialOrd> Contains<T> for Range<T> {
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
