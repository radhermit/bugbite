use serde::{Deserialize, Deserializer};

/// Deserialize an empty string as None.
pub(crate) fn non_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d).map(|o| o.filter(|s: &String| !s.is_empty()))
}

/// Deserialize a null value into an empty vector.
pub(crate) fn null_empty_vec<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    Option::deserialize(d).map(|o| o.unwrap_or_default())
}
