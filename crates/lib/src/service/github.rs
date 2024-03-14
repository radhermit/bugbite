use std::fmt;

use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::traits::{NullRequest, ServiceParams, WebClient, WebService};
use crate::Error;

use super::ServiceKind;

mod get;
pub mod modify;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    pub token: Option<String>,
    cache: ServiceCache,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            token: None,
            cache: Default::default(),
        })
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::Github
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.kind(), self.base())
    }
}

// TODO: remove this once authentication support is added
#[derive(Debug)]
pub struct Service {
    config: Config,
    client: reqwest::Client,
}

impl Service {
    pub(crate) fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            client: builder.build()?,
        })
    }
}

impl<'a> WebClient<'a> for Service {
    type Service = Self;
    type ModifyParams = modify::ModifyParams<'a>;
    type SearchQuery = search::QueryBuilder<'a>;

    fn service(&self) -> &Self::Service {
        self
    }

    fn modify_params(&'a self) -> Self::ModifyParams {
        Self::ModifyParams::new(self.service())
    }

    fn search_query(&'a self) -> Self::SearchQuery {
        Self::SearchQuery::new(self.service())
    }
}

impl<'a> WebService<'a> for Service {
    const API_VERSION: &'static str = "2022-11-28";
    type Response = serde_json::Value;
    type GetRequest = get::GetRequest;
    type ModifyRequest = NullRequest;
    type SearchRequest = search::SearchRequest;

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
