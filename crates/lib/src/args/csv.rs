use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use itertools::Itertools;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

/// Supported service variants
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub struct Csv<T: fmt::Display + FromStr> {
    values: Vec<T>,
}

impl<T: fmt::Display + FromStr> Csv<T> {
    pub fn into_inner(self) -> Vec<T> {
        self.values
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

        let mut values = vec![];
        for item in items {
            let value = item
                .parse()
                .map_err(|e| Error::InvalidValue(format!("{item}: {e}")))?;
            values.push(value);
        }

        Ok(Self { values })
    }
}

impl<T: fmt::Display + FromStr> fmt::Display for Csv<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.values.iter().join(","))
    }
}

impl<T: fmt::Display + FromStr> IntoIterator for Csv<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<T: fmt::Display + FromStr> Deref for Csv<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.values.deref()
    }
}

impl<T: fmt::Display + FromStr> DerefMut for Csv<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.values.deref_mut()
    }
}
