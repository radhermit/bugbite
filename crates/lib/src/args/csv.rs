use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use itertools::Itertools;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

/// Comma-separated value support.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub struct Csv<T: fmt::Display + FromStr>(Vec<T>);

impl<T: fmt::Display + FromStr> Csv<T> {
    /// Convert into the vector of comma-separated values.
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T> FromStr for Csv<T>
where
    T: fmt::Display + FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let items: Vec<_> = s.split(',').filter(|x| !x.is_empty()).collect();
        if items.is_empty() {
            return Err(Error::InvalidValue("empty Csv string".to_string()));
        }

        let values = items
            .into_iter()
            .map(|x| {
                x.parse()
                    .map_err(|e| Error::InvalidValue(format!("{x}: {e}")))
            })
            .try_collect()?;

        Ok(Self(values))
    }
}

impl<T: fmt::Display + FromStr> fmt::Display for Csv<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.iter().join(","))
    }
}

impl<T: fmt::Display + FromStr> IntoIterator for Csv<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T: fmt::Display + FromStr> Deref for Csv<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0.deref()
    }
}

impl<T: fmt::Display + FromStr> DerefMut for Csv<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.0.deref_mut()
    }
}
