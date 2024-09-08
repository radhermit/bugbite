use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use ordered_multimap::ListOrderedMultimap;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::traits::Api;
use crate::Error;

#[derive(Debug, Default)]
pub(crate) struct Query(ListOrderedMultimap<String, String>);

impl Query {
    /// Appends a value to the list of values associated with the given key.
    pub(crate) fn append<K, V>(&mut self, key: K, value: V)
    where
        K: Api,
        V: Api,
    {
        self.0.append(key.api(), value.api());
    }

    /// Inserts a value overriding entries associated with the given key.
    pub(crate) fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Api,
        V: Api,
    {
        self.0.insert(key.api(), value.api());
    }
}

impl Deref for Query {
    type Target = ListOrderedMultimap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Invertible search order sorting term.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Order<T> {
    Ascending(T),
    Descending(T),
}

impl<T: FromStr> TryFrom<&str> for Order<T> {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<T: FromStr> FromStr for Order<T> {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let field = |s: &str| -> crate::Result<T> {
            s.parse()
                .map_err(|_| Error::InvalidValue(format!("unknown field: {s}")))
        };

        let value = if let Some(value) = s.strip_prefix('-') {
            let field = field(value)?;
            Self::Descending(field)
        } else {
            let value = s.strip_prefix('+').unwrap_or(s);
            let field = field(value)?;
            Self::Ascending(field)
        };

        Ok(value)
    }
}

impl<T: fmt::Display> fmt::Display for Order<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Ascending(value) => write!(f, "{value}"),
            Self::Descending(value) => write!(f, "-{value}"),
        }
    }
}
