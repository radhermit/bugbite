use std::fmt;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::traits::{Merge, MergeOption, WebClient, WebService};
use crate::Error;

use super::{ClientParameters, ServiceKind};

mod get;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct Authentication {
    pub user: Option<String>,
    pub token: Option<String>,
}

impl Merge for Authentication {
    fn merge(&mut self, other: Self) {
        *self = Self {
            user: self.user.merge(other.user),
            token: self.token.merge(other.token),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Config {
    base: Url,
    pub name: String,
    #[serde(flatten)]
    pub auth: Authentication,
    #[serde(flatten)]
    pub client: ClientParameters,
}

impl Config {
    pub(super) fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            name: Default::default(),
            auth: Default::default(),
            client: Default::default(),
        })
    }
}

impl WebClient for Config {
    fn base(&self) -> &Url {
        &self.base
    }

    fn kind(&self) -> ServiceKind {
        ServiceKind::Github
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// TODO: remove this once authentication support is added
#[derive(Debug)]
pub struct Service {
    config: Config,
    _cache: ServiceCache,
    _client: reqwest::Client,
}

impl PartialEq for Service {
    fn eq(&self, other: &Self) -> bool {
        self.config == other.config
    }
}

impl Service {
    /// Create a new Service from a given base URL.
    pub fn new(base: &str) -> crate::Result<Self> {
        let config = Config::new(base)?;
        Self::from_config(config)
    }

    /// Create a new Service from a Config.
    pub fn from_config(config: Config) -> crate::Result<Self> {
        let _client = config.client.build()?;
        Ok(Self {
            config,
            _cache: Default::default(),
            _client,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: std::fmt::Display>(&self, id: I) -> String {
        let base = self.base().as_str().trim_end_matches('/');
        format!("{base}/issues/{id}")
    }

    pub fn get<I>(&self, _ids: I) -> get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        todo!("get requests unsupported")
    }

    pub fn search(&self) -> search::Request {
        search::Request::new(self)
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

    fn inject_auth(
        &self,
        _request: RequestBuilder,
        _required: bool,
    ) -> crate::Result<RequestBuilder> {
        unimplemented!("authentication unsupported")
    }

    async fn parse_response(&self, _response: reqwest::Response) -> crate::Result<Self::Response> {
        unimplemented!("request parsing unsupported")
    }
}

impl WebClient for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn name(&self) -> &str {
        self.config.name()
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {}
