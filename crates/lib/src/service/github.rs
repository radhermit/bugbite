use std::fmt;
use std::sync::Arc;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::traits::{Merge, MergeOption, WebClient, WebService};
use crate::Error;

use super::{Client, ClientParameters, ServiceKind};

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
    pub fn new(base: &str) -> crate::Result<Self> {
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

#[derive(Debug)]
struct Service {
    config: Config,
    _cache: ServiceCache,
    client: Client,
}

#[derive(Debug)]
pub struct ServiceBuilder(Config);

impl ServiceBuilder {
    pub fn auth(mut self, value: Authentication) -> Self {
        self.0.auth.merge(value);
        self
    }

    pub fn client(mut self, value: ClientParameters) -> Self {
        self.0.client.merge(value);
        self
    }

    /// Create a new service.
    pub fn build(self) -> crate::Result<Github> {
        let client = self.0.client.build()?;
        Ok(Github(Arc::new(Service {
            config: self.0,
            _cache: Default::default(),
            client,
        })))
    }
}

#[derive(Debug, Clone)]
pub struct Github(Arc<Service>);

impl PartialEq for Github {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl fmt::Display for Github {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

impl Github {
    /// Create a new Service using a given base URL.
    pub fn new(base: &str) -> crate::Result<Self> {
        Self::builder(base)?.build()
    }

    /// Create a new Service builder using a given base URL.
    pub fn builder(base: &str) -> crate::Result<ServiceBuilder> {
        Ok(ServiceBuilder(Config::new(base)?))
    }

    /// Create a new Service builder using a given base URL.
    pub fn config_builder(
        config: &crate::config::Config,
        name: Option<&str>,
    ) -> crate::Result<ServiceBuilder> {
        let config = config
            .get_kind(ServiceKind::Github, name)?
            .into_github()
            .unwrap();
        Ok(ServiceBuilder(config))
    }

    pub fn config(&self) -> &Config {
        &self.0.config
    }

    pub fn client(&self) -> &Client {
        &self.0.client
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

impl WebService for Github {
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

impl WebClient for Github {
    fn base(&self) -> &Url {
        self.config().base()
    }

    fn kind(&self) -> ServiceKind {
        self.config().kind()
    }

    fn name(&self) -> &str {
        self.config().name()
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {}
