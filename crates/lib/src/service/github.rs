use std::fmt;

use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::traits::{NullRequest, WebService};
use crate::Error;

use super::ServiceKind;

mod get;
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

// TODO: remove this once authentication support is added
#[derive(Debug)]
pub struct Service {
    config: Config,
    _client: reqwest::Client,
}

impl Service {
    pub(crate) fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            _client: builder.build()?,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: std::fmt::Display>(&self, id: I) -> String {
        let base = self.base().as_str().trim_end_matches('/');
        format!("{base}/issues/{id}")
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

impl<'a> WebService<'a> for Service {
    const API_VERSION: &'static str = "2022-11-28";
    type Response = serde_json::Value;
    type GetRequest = get::GetRequest;
    type CreateRequest = NullRequest;
    type CreateParams = ();
    type UpdateRequest = NullRequest;
    type UpdateParams = ();
    type SearchRequest = search::SearchRequest;
    type SearchParams = search::Parameters;

    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {}
