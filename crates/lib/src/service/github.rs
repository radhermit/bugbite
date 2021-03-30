use std::fmt;
use std::str::FromStr;

use ordered_multimap::ListOrderedMultimap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::traits::{Params, WebService};
use crate::Error;

use super::ServiceKind;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    cache: ServiceCache,
}

impl Config {
    pub(super) fn new(base: Url) -> Self {
        Self {
            base,
            cache: Default::default(),
        }
    }

    pub(super) fn service(self, client: Client) -> Service {
        Service {
            config: self,
            client,
        }
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::Github
    }
}

pub struct Service {
    config: Config,
    client: reqwest::Client,
}

impl WebService for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {}

/// Construct a search query.
#[derive(Debug, Default)]
pub struct QueryBuilder {
    query: ListOrderedMultimap<String, String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
    {
        self.query.append(key.to_string(), value.to_string());
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
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

impl Params for QueryBuilder {
    fn params(&mut self) -> String {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        params.extend_pairs(self.query.iter());
        params.finish()
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
    Reactions,
    Interactions,
    Created,
    Updated,
}
