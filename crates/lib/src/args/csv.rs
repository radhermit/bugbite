use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use itertools::Itertools;
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::Error;

/// Comma-separated value support.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
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
        let values: Vec<_> = s
            .split(',')
            .filter(|x| !x.is_empty())
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

impl<T, U> PartialEq<Vec<U>> for Csv<T>
where
    T: fmt::Display + FromStr + PartialEq<U>,
{
    fn eq(&self, other: &Vec<U>) -> bool {
        &self.0 == other
    }
}

impl<T, U> PartialEq<[U]> for Csv<T>
where
    T: fmt::Display + FromStr + PartialEq<U>,
{
    fn eq(&self, other: &[U]) -> bool {
        self.0 == other
    }
}

impl<T, U, const N: usize> PartialEq<[U; N]> for Csv<T>
where
    T: fmt::Display + FromStr + PartialEq<U>,
{
    fn eq(&self, other: &[U; N]) -> bool {
        self.0 == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        for (value, parsed, display) in [
            ("", vec![], ""),
            (",", vec![], ""),
            (",,", vec![], ""),
            ("a", vec!["a"], "a"),
            ("a,b", vec!["a", "b"], "a,b"),
        ] {
            let csv: Csv<String> = value.parse().unwrap();
            assert_eq!(&csv, parsed.as_slice());
            assert_eq!(csv.to_string(), display);
            assert_eq!(csv.into_inner(), parsed.as_slice());
        }
    }

    #[test]
    fn deref() {
        let mut csv: Csv<String> = "a,b".parse().unwrap();
        // immutable deref
        assert_eq!(&csv, &["a", "b"]);
        assert_eq!(csv.len(), 2);
        // mutable deref
        csv[1] = "c".to_string();
        assert_eq!(&csv, &["a", "c"]);
    }
}
