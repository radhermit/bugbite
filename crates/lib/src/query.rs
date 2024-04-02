use std::fmt;
use std::str::FromStr;

use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::VariantNames;

use crate::traits::Api;
use crate::Error;

#[derive(Debug, Default)]
pub(crate) struct QueryBuilder {
    query: ListOrderedMultimap<String, String>,
}

impl QueryBuilder {
    pub(crate) fn encode(&self) -> String {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        params.extend_pairs(self.query.iter());
        params.finish()
    }

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum OrderType {
    Ascending,
    Descending,
}

/// Invertable search order sorting term.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Order<T> {
    pub(crate) order: OrderType,
    pub(crate) field: T,
}

impl<T> Order<T> {
    pub fn ascending(field: T) -> Self {
        Self {
            order: OrderType::Ascending,
            field,
        }
    }

    pub fn descending(field: T) -> Self {
        Self {
            order: OrderType::Descending,
            field,
        }
    }
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
        let (order, field) = if let Some(value) = s.strip_prefix('-') {
            (OrderType::Descending, value)
        } else {
            (OrderType::Ascending, s.strip_prefix('+').unwrap_or(s))
        };
        let field = field.parse().map_err(|_| {
            let possible = T::VARIANTS.iter().join(", ");
            Error::InvalidValue(format!(
                "unknown search field: {field}\n  [possible values: {possible}]"
            ))
        })?;
        Ok(Self { order, field })
    }
}

impl<T: fmt::Display> fmt::Display for Order<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.order {
            OrderType::Descending => write!(f, "-{}", self.field),
            OrderType::Ascending => write!(f, "{}", self.field),
        }
    }
}
