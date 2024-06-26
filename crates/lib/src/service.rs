use std::fmt;
use std::time::Duration;

use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{AsRefStr, Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::traits::WebClient;
use crate::Error;

pub mod bugzilla;
pub mod github;
pub mod redmine;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Supported service variants
#[derive(
    AsRefStr,
    Display,
    EnumIter,
    EnumString,
    VariantNames,
    DeserializeFromStr,
    SerializeDisplay,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Copy,
    Clone,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ServiceKind {
    /// Targets the REST API v1 provided by bugzilla-5.0 and up.
    /// API docs: https://bugzilla.readthedocs.io/en/latest/api/core/v1/
    #[default]
    Bugzilla,

    /// Targets the GitHub REST API version 2022-11-28.
    /// API docs: https://docs.github.com/en/rest/about-the-rest-api
    Github,

    /// Targets the REST API using the JSON format.
    /// API docs: https://www.redmine.org/projects/redmine/wiki/rest_api
    Redmine,
}

#[derive(EnumAsInner, Deserialize, Serialize, Debug, Clone)]
pub enum Config {
    Bugzilla(bugzilla::Config),
    Github(github::Config),
    Redmine(redmine::Config),
}

impl Config {
    pub fn new(kind: ServiceKind, base: &str) -> crate::Result<Self> {
        match kind {
            ServiceKind::Bugzilla => Ok(Config::Bugzilla(bugzilla::Config::new(base)?)),
            ServiceKind::Github => Ok(Config::Github(github::Config::new(base)?)),
            ServiceKind::Redmine => Ok(Config::Redmine(redmine::Config::new(base)?)),
        }
    }

    pub fn base(&self) -> &Url {
        match self {
            Self::Bugzilla(config) => config.base(),
            Self::Github(config) => config.base(),
            Self::Redmine(config) => config.base(),
        }
    }

    pub fn kind(&self) -> ServiceKind {
        match self {
            Self::Bugzilla(config) => config.kind(),
            Self::Github(config) => config.kind(),
            Self::Redmine(config) => config.kind(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} -- {}", self.kind(), self.base())
    }
}

#[derive(Debug)]
pub struct ClientBuilder {
    timeout: f64,
    insecure: bool,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            timeout: 30.0,
            insecure: false,
        }
    }
}

impl ClientBuilder {
    pub fn timeout(mut self, timeout: f64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }

    pub fn build(self) -> crate::Result<reqwest::Client> {
        reqwest::Client::builder()
            .hickory_dns(true)
            .use_rustls_tls()
            .user_agent(USER_AGENT)
            // TODO: switch to cookie_provider() once cookie (de)serialization is supported
            .cookie_store(true)
            .timeout(Duration::from_secs_f64(self.timeout))
            .danger_accept_invalid_certs(self.insecure)
            .build()
            .map_err(|e| Error::InvalidValue(format!("failed creating client: {e}")))
    }
}

#[derive(EnumAsInner, Debug)]
pub enum Service {
    Bugzilla(bugzilla::Service),
    Github(github::Service),
    Redmine(redmine::Service),
}

impl Service {
    pub fn base(&self) -> &Url {
        match self {
            Self::Bugzilla(service) => service.base(),
            Self::Github(service) => service.base(),
            Self::Redmine(service) => service.base(),
        }
    }

    pub fn kind(&self) -> ServiceKind {
        match self {
            Self::Bugzilla(service) => service.kind(),
            Self::Github(service) => service.kind(),
            Self::Redmine(service) => service.kind(),
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.base(), self.kind())
    }
}
