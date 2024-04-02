use std::fmt;
use std::str::FromStr;

use itertools::Itertools;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

use super::MaybeStdinVec;

#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
pub enum ExistsOrValues<T> {
    Exists(bool),
    Values(Vec<T>),
}

impl<T> ExistsOrValues<MaybeStdinVec<T>> {
    pub fn flatten(self) -> ExistsOrValues<T> {
        match self {
            Self::Exists(value) => ExistsOrValues::Exists(value),
            Self::Values(values) => ExistsOrValues::Values(values.into_iter().flatten().collect()),
        }
    }
}

impl<T> FromStr for ExistsOrValues<T>
where
    T: FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(Self::Exists(true)),
            "false" => Ok(Self::Exists(false)),
            value => Ok(Self::Values(
                value
                    .split(',')
                    .map(|x| {
                        x.parse()
                            .map_err(|e| Error::InvalidValue(format!("failed parsing: {e}")))
                    })
                    .try_collect()?,
            )),
        }
    }
}

impl<T: fmt::Display> fmt::Display for ExistsOrValues<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Exists(value) => value.fmt(f),
            Self::Values(values) => values.iter().join(",").fmt(f),
        }
    }
}
