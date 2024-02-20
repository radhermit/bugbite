use std::fmt;
use std::str::FromStr;

use base64::prelude::*;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

pub mod bugzilla;
pub mod github;

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
}

/// Raw binary data encoded as Base64.
#[derive(DeserializeFromStr, SerializeDisplay, Default, Debug, Eq, PartialEq)]
pub(crate) struct Base64(Vec<u8>);

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
