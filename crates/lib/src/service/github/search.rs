use std::fmt;
use std::str::FromStr;

use ordered_multimap::ListOrderedMultimap;
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::{Query, Request, ServiceParams};
use crate::Error;

/// Construct a search query.
#[derive(Debug)]
pub struct QueryBuilder<'a> {
    _service: &'a super::Service,
    query: ListOrderedMultimap<String, String>,
}

impl<'a> ServiceParams<'a> for QueryBuilder<'a> {
    type Service = super::Service;

    fn new(_service: &'a Self::Service) -> Self {
        Self {
            _service,
            query: Default::default(),
        }
    }
}

impl QueryBuilder<'_> {
    pub fn append<K, V>(&mut self, key: K, value: V)
    where
        K: fmt::Display,
        V: fmt::Display,
    {
        self.query.append(key.to_string(), value.to_string());
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: fmt::Display,
        V: fmt::Display,
    {
        self.query.insert(key.to_string(), value.to_string());
    }

    pub fn sort(&mut self, order: SearchOrder) {
        self.insert("sort", order.term);
        if order.descending {
            self.insert("order", "desc");
        } else {
            self.insert("order", "asc");
        }
    }
}

impl Query for QueryBuilder<'_> {
    fn params(&mut self) -> crate::Result<String> {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        params.extend_pairs(self.query.iter());
        Ok(params.finish())
    }
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

pub(crate) struct SearchRequest(reqwest::Request);

impl Request for SearchRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
