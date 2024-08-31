use std::fmt;
use std::str::FromStr;

use ordered_multimap::ListOrderedMultimap;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::VariantNames;

use crate::traits::Api;
use crate::Error;

#[derive(Debug, Default)]
pub(crate) struct Query {
    query: ListOrderedMultimap<String, String>,
}

impl Query {
    pub(crate) fn append<K, V>(&mut self, key: K, value: V)
    where
        K: Api,
        V: Api,
    {
        self.query.append(key.api(), value.api());
    }

    pub(crate) fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Api,
        V: Api,
    {
        self.query.insert(key.api(), value.api());
    }
}

impl<'a> IntoIterator for &'a Query {
    type Item = (&'a String, &'a String);
    type IntoIter = ordered_multimap::list_ordered_multimap::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.query.iter()
    }
}

/// Invertible search order sorting term.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Order<T> {
    Ascending(T),
    Descending(T),
}

impl<T: FromStr + VariantNames> TryFrom<&str> for Order<T> {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<T: FromStr + VariantNames> FromStr for Order<T> {
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
