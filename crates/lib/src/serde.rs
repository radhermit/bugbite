use serde::{Deserialize, Deserializer};

use crate::types::{Ordered, OrderedSet};

/// Deserialize an empty string as None.
pub(crate) fn non_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d).map(|o| o.filter(|s: &String| !s.is_empty()))
}

/// Deserialize a null value into an empty string.
pub(crate) fn null_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Option::deserialize(d).map(|o| o.unwrap_or_default())
}

/// Deserialize a null value into an empty vector.
pub(crate) fn null_empty_vec<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    Option::deserialize(d).map(|o| o.unwrap_or_default())
}

/// Deserialize a null value into an empty set.
pub(crate) fn null_empty_set<'de, D: Deserializer<'de>, T: Deserialize<'de> + Ordered>(
    d: D,
) -> Result<OrderedSet<T>, D::Error> {
    Option::deserialize(d).map(|o| o.unwrap_or_default())
}
