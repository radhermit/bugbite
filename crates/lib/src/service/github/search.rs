use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::RequestSend;
use crate::Error;

/// Issue search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Parameters {
    pub order: Option<SearchOrder>,
}

/// Invertable search order sorting term.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub struct SearchOrder {
    descending: bool,
    term: SearchTerm,
}

impl FromStr for SearchOrder {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let term = s.strip_prefix('-').unwrap_or(s);
        let descending = term != s;
        let term = term
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown search term: {term}")))?;
        Ok(Self { descending, term })
    }
}

impl fmt::Display for SearchOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = &self.term;
        if self.descending {
            write!(f, "-{name}")
        } else {
            write!(f, "{name}")
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone)]
#[strum(serialize_all = "kebab-case")]
pub enum SearchTerm {
    Comments,
    Created,
    Interactions,
    Reactions,
    Updated,
}

pub struct SearchRequest(reqwest::Request);

impl RequestSend for SearchRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
