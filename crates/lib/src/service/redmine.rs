use std::fmt;
use std::sync::{Arc, OnceLock};

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::Error;
use crate::traits::{Merge, MergeOption, WebClient, WebService};

use super::{Client, ClientParameters, ServiceKind};

pub mod get;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct Authentication {
    pub key: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
}

impl Merge for Authentication {
    fn merge(&mut self, other: Self) {
        *self = Self {
            key: self.key.merge(other.key),
            user: self.user.merge(other.user),
            password: self.password.merge(other.password),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Config {
    base: Url,
    pub name: String,
    #[serde(skip)]
    web_base: OnceLock<Option<Url>>,
    #[serde(flatten)]
    pub auth: Authentication,
    #[serde(flatten)]
    pub client: ClientParameters,
    pub max_search_results: Option<usize>,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            web_base: Default::default(),
            name: Default::default(),
            auth: Default::default(),
            client: Default::default(),
            max_search_results: Default::default(),
        })
    }

    /// Return the base URL for the service, removing any project subpath if it exists.
    fn web_base(&self) -> &Url {
        self.web_base
            .get_or_init(|| {
                if let Some((base, _project)) = self.base.as_str().split_once("/projects/") {
                    if let Ok(url) = Url::parse(base) {
                        return Some(url);
                    }
                }
                None
            })
            .as_ref()
            .unwrap_or(&self.base)
    }

    /// Maximum number of results that can be returned by a search request.
    ///
    /// Fallback to redmine's internal default of 100.
    fn max_search_results(&self) -> usize {
        match self.max_search_results.unwrap_or_default() {
            0 => 100,
            n => n,
        }
    }
}

impl WebClient for Config {
    fn base(&self) -> &Url {
        &self.base
    }

    fn kind(&self) -> ServiceKind {
        ServiceKind::Redmine
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// TODO: remove this once authentication support is added
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
    pub fn build(self) -> crate::Result<Redmine> {
        let client = self.0.client.build()?;
        Ok(Redmine(Arc::new(Service {
            config: self.0,
            _cache: Default::default(),
            client,
        })))
    }
}

#[derive(Debug, Clone)]
pub struct Redmine(Arc<Service>);

impl PartialEq for Redmine {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl fmt::Display for Redmine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

impl Redmine {
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
            .get_kind(ServiceKind::Redmine, name)?
            .into_redmine()
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
    pub fn item_url<I: fmt::Display>(&self, id: I) -> String {
        let base = self.config().web_base().as_str().trim_end_matches('/');
        format!("{base}/issues/{id}")
    }

    pub fn get<I>(&self, ids: I) -> get::Request
    where
        I: IntoIterator<Item = u64>,
    {
        get::Request::new(self, ids)
    }

    pub fn search(&self) -> search::Request {
        search::Request::new(self)
    }
}

impl WebService for Redmine {
    const API_VERSION: &'static str = "5.1";
    type Response = serde_json::Value;

    fn inject_auth(
        &self,
        request: RequestBuilder,
        required: bool,
    ) -> crate::Result<RequestBuilder> {
        let auth = &self.config().auth;
        if let Some(key) = auth.key.as_ref() {
            Ok(request.header("X-Redmine-API-Key", key))
        } else if let (Some(user), Some(pass)) = (&auth.user, &auth.password) {
            Ok(request.basic_auth(user, Some(pass)))
        } else if !required {
            Ok(request)
        } else {
            Err(Error::Auth)
        }
    }

    async fn parse_response(&self, response: reqwest::Response) -> crate::Result<Self::Response> {
        trace!("{response:?}");
        match response.error_for_status_ref() {
            Ok(_) => {
                let mut data: serde_json::Value = response.json().await?;
                debug!(
                    "response data:\n{}",
                    serde_json::to_string_pretty(&data).unwrap()
                );
                let errors = data["errors"].take();
                if !errors.is_null() {
                    let errors: Vec<_> = serde_json::from_value(errors).map_err(|e| {
                        Error::InvalidValue(format!("failed deserializing errors: {e}"))
                    })?;
                    let error = errors.into_iter().next().unwrap();
                    Err(Error::Redmine(error))
                } else {
                    Ok(data)
                }
            }
            Err(e) => {
                if let Ok(mut data) = response.json::<serde_json::Value>().await {
                    debug!("error:\n{}", serde_json::to_string_pretty(&data).unwrap());
                    let errors = data["errors"].take();
                    if !errors.is_null() {
                        let errors: Vec<_> = serde_json::from_value(errors).map_err(|e| {
                            Error::InvalidValue(format!("failed deserializing errors: {e}"))
                        })?;
                        let error = errors.into_iter().next().unwrap();
                        return Err(Error::Redmine(error));
                    }
                }
                Err(e.into())
            }
        }
    }
}

impl WebClient for Redmine {
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

#[derive(Display, EnumString, VariantNames, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum IssueField {
    /// person the issue is assigned to
    Assignee,
    /// person who created the issue
    Author,
    /// time when the issue was closed
    Closed,
    /// time when the issue was created
    Created,
    /// issue ID
    Id,
    /// issue priority
    Priority,
    /// issue status
    Status,
    /// issue subject
    Subject,
    /// issue type
    Tracker,
    /// time when the issue was last updated
    Updated,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {}
