use std::str::FromStr;

use itertools::Itertools;

#[derive(Debug, Clone)]
pub(crate) enum ExistsOrArray<T> {
    Exists(bool),
    Array(Vec<T>),
}

impl<T> FromStr for ExistsOrArray<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(ExistsOrArray::Exists(true)),
            "false" => Ok(ExistsOrArray::Exists(false)),
            value => Ok(ExistsOrArray::Array(
                value
                    .split(',')
                    .map(|x| {
                        x.parse()
                            .map_err(|e| anyhow::anyhow!("failed parsing: {e}"))
                    })
                    .try_collect()?,
            )),
        }
    }
}
