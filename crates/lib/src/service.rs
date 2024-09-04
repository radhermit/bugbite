use std::time::Duration;
use std::{fmt, fs};

use camino::Utf8PathBuf;
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
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Config {
    Bugzilla(bugzilla::Config),
    Github(github::Config),
    Redmine(redmine::Config),
}

impl Config {
    pub fn new(kind: ServiceKind, base: &str) -> crate::Result<Self> {
        let config = match kind {
            ServiceKind::Bugzilla => Self::Bugzilla(bugzilla::Config::new(base)?),
            ServiceKind::Github => Self::Github(github::Config::new(base)?),
            ServiceKind::Redmine => Self::Redmine(redmine::Config::new(base)?),
        };

        Ok(config)
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
    pub certificate: Option<Utf8PathBuf>,
    pub insecure: bool,
    pub timeout: f64,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            certificate: None,
            insecure: false,
            timeout: 30.0,
        }
    }
}

impl ClientBuilder {
    fn build(&self) -> crate::Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder()
            // TODO: switch to cookie_provider() once cookie (de)serialization is supported
            .cookie_store(true)
            .danger_accept_invalid_certs(self.insecure)
            .hickory_dns(true)
            .timeout(Duration::from_secs_f64(self.timeout))
            .use_rustls_tls()
            .user_agent(USER_AGENT);

        if let Some(path) = self.certificate.as_deref() {
            let data = fs::read(path).map_err(|e| {
                Error::InvalidValue(format!("failed reading certificate: {path}: {e}"))
            })?;
            let cert = reqwest::tls::Certificate::from_pem(&data)
                .map_err(|e| Error::InvalidValue(format!("invalid certificate: {path}: {e}")))?;
            builder = builder.add_root_certificate(cert);
        }

        builder
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
