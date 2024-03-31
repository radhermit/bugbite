use std::str::FromStr;

use itertools::Itertools;

#[derive(Debug, Clone)]
pub(crate) enum ExistsOrValues<T> {
    Exists(bool),
    Values(Vec<T>),
}

impl<T> FromStr for ExistsOrValues<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(Self::Exists(true)),
            "false" => Ok(Self::Exists(false)),
            value => Ok(Self::Values(
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
