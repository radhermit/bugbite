use serde::{Deserialize, Serialize};
use url::Url;

use crate::traits::WebService;

use super::ServiceKind;

mod get;
pub mod search;

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

    pub(crate) fn service(self, client: reqwest::Client) -> Service {
        Service {
            config: self,
            token: None,
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

// TODO: remove this once authentication support is added
#[allow(dead_code)]
#[derive(Debug)]
pub struct Service {
    config: Config,
    token: Option<String>,
    client: reqwest::Client,
}

impl WebService for Service {
    const API_VERSION: &'static str = "2022-11-28";
    type Response = serde_json::Value;
    type GetRequest = get::GetRequest;
    type SearchRequest = search::SearchRequest;
    type SearchQuery = search::QueryBuilder;

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
