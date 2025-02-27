use std::fmt;
use std::str::FromStr;

use crate::Error;

mod exists;
pub use exists::ExistsOrValues;
mod csv;
pub use csv::Csv;
pub(crate) mod maybe_stdin;
pub use maybe_stdin::{MaybeStdin, MaybeStdinVec};

/// Argument that pulls from standard input when "-" or uses comma-separated values.
#[derive(Debug, Clone)]
pub enum CsvOrStdin<T: fmt::Display + FromStr> {
    Csv(Csv<T>),
    Stdin(MaybeStdinVec<T>),
}

impl<T: fmt::Display + FromStr> CsvOrStdin<T> {
    /// Convert into the vector of values.
    pub fn into_inner(self) -> Vec<T> {
        match self {
            Self::Csv(x) => x.into_inner(),
            Self::Stdin(x) => x.into_inner(),
        }
    }

    /// Return the iterator of values.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            Self::Csv(x) => x.iter(),
            Self::Stdin(x) => x.iter(),
        }
    }
}

impl<T> FromStr for CsvOrStdin<T>
where
    T: fmt::Display + FromStr,
    T::Err: fmt::Display,
{
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if s == "-" {
            Ok(Self::Stdin(
                s.parse().map_err(|e| Error::InvalidValue(format!("{e}")))?,
            ))
        } else {
            Ok(Self::Csv(s.parse()?))
        }
    }
}

impl<T: fmt::Display + FromStr> IntoIterator for CsvOrStdin<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Csv(x) => x.into_iter(),
            Self::Stdin(x) => x.into_iter(),
        }
    }
}

impl<'a, T: fmt::Display + FromStr> IntoIterator for &'a CsvOrStdin<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CsvOrStdin::Csv(x) => x.iter(),
            CsvOrStdin::Stdin(x) => x.iter(),
        }
    }
}
