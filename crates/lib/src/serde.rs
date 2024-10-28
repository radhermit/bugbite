use byte_unit::Byte;
use serde::{Deserialize, Deserializer};

/// Deserialize number as a Byte object.
pub(crate) fn byte_object<'de, D: Deserializer<'de>>(d: D) -> Result<Byte, D::Error> {
    u64::deserialize(d).map(Byte::from_u64)
}

/// Deserialize an empty string as None.
pub(crate) fn non_empty_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d).map(|o| o.filter(|s: &String| !s.is_empty()))
}
