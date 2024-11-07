use std::fs;
use std::ops::Deref;
use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{AsRefStr, Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::traits::{Merge, MergeOption, WebClient};
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
    pub(super) fn new(kind: ServiceKind, base: &str) -> crate::Result<Self> {
        let service = match kind {
            ServiceKind::Bugzilla => Self::Bugzilla(bugzilla::Config::new(base)?),
            ServiceKind::Github => Self::Github(github::Config::new(base)?),
            ServiceKind::Redmine => Self::Redmine(redmine::Config::new(base)?),
        };

        Ok(service)
    }

    pub(super) fn try_from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading config: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing config: {path}: {e}")))
    }

    pub(super) fn merge(&mut self, value: ClientParameters) {
        match self {
            Self::Bugzilla(config) => config.client.merge(value),
            Self::Github(config) => config.client.merge(value),
            Self::Redmine(config) => config.client.merge(value),
        }
    }
}

impl WebClient for Config {
    fn base(&self) -> &Url {
        match self {
            Self::Bugzilla(config) => config.base(),
            Self::Github(config) => config.base(),
            Self::Redmine(config) => config.base(),
        }
    }

    fn kind(&self) -> ServiceKind {
        match self {
            Self::Bugzilla(config) => config.kind(),
            Self::Github(config) => config.kind(),
            Self::Redmine(config) => config.kind(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Bugzilla(config) => config.name(),
            Self::Github(config) => config.name(),
            Self::Redmine(config) => config.name(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct ClientParameters {
    pub certificate: Option<Utf8PathBuf>,
    pub concurrent: Option<usize>,
    pub insecure: Option<bool>,
    pub proxy: Option<String>,
    pub timeout: Option<f64>,
}

impl Merge for ClientParameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            certificate: self.certificate.merge(other.certificate),
            concurrent: self.concurrent.merge(other.concurrent),
            insecure: self.insecure.merge(other.insecure),
            proxy: self.proxy.merge(other.proxy),
            timeout: self.timeout.merge(other.timeout),
        }
    }
}

impl ClientParameters {
    fn build(&self) -> crate::Result<Client> {
        let mut builder = reqwest::Client::builder()
            // TODO: switch to cookie_provider() once cookie (de)serialization is supported
            .cookie_store(true)
            .danger_accept_invalid_certs(self.insecure.unwrap_or_default())
            .hickory_dns(true)
            .pool_max_idle_per_host(self.concurrent.unwrap_or(3))
            .timeout(Duration::from_secs_f64(self.timeout.unwrap_or(30.0)))
            .user_agent(USER_AGENT);

        // force rustls usage when enabled
        if cfg!(feature = "rustls-tls") {
            builder = builder.use_rustls_tls();
        }

        if let Some(proxy) = &self.proxy {
            let url = Url::parse(proxy)
                .map_err(|e| Error::InvalidValue(format!("invalid proxy URL: {e}")))?;
            let proxy = reqwest::Proxy::all(url)
                .map_err(|_| Error::InvalidValue(format!("invalid proxy URL: {proxy}")))?;
            builder = builder.proxy(proxy);
        }

        if let Some(path) = &self.certificate {
            let data = fs::read(path).map_err(|e| {
                Error::InvalidValue(format!("failed reading certificate: {path}: {e}"))
            })?;
            let cert = reqwest::tls::Certificate::from_pem(&data)
                .map_err(|e| Error::InvalidValue(format!("invalid certificate: {path}: {e}")))?;
            builder = builder.add_root_certificate(cert);
        }

        let client = builder
            .build()
            .map_err(|e| Error::InvalidValue(format!("failed creating client: {e}")))?;

        Ok(Client {
            params: self.clone(),
            client,
        })
    }
}

#[derive(Debug)]
pub struct Client {
    pub params: ClientParameters,
    client: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        ClientParameters::default().build().unwrap()
    }
}

impl Deref for Client {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
