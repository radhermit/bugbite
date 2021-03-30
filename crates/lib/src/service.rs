use std::fmt;

use reqwest::{Client, Request};
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{AsRefStr, Display, EnumIter, EnumString, VariantNames};
use url::Url;

pub mod bugzilla;
pub mod github;

use crate::traits::{Params, WebService};
use crate::Error;

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
    Copy,
    Clone,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ServiceKind {
    /// Targets the REST API v1 provided by bugzilla-5.0 and up.
    /// API docs: https://bugzilla.readthedocs.io/en/latest/api/
    BugzillaRestV1,
    Github,
}

impl ServiceKind {
    /// Create a new service using a given base URL.
    pub fn create(self, base: &str) -> crate::Result<Config> {
        let base = Url::parse(base)
            .map_err(|e| Error::InvalidValue(format!("invalid {self} URL: {base}: {e}")))?;

        let config = match self {
            Self::BugzillaRestV1 => Config::BugzillaRestV1(bugzilla::Config::new(base)),
            Self::Github => Config::Github(github::Config::new(base)),
        };

        Ok(config)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Config {
    BugzillaRestV1(bugzilla::Config),
    Github(github::Config),
}

impl Config {
    pub(crate) fn service(self, client: Client) -> Service {
        match self {
            Self::BugzillaRestV1(config) => Service::BugzillaRestV1(config.service(client)),
            Self::Github(config) => Service::Github(config.service(client)),
        }
    }

    fn base(&self) -> &Url {
        match self {
            Self::BugzillaRestV1(config) => config.base(),
            Self::Github(config) => config.base(),
        }
    }

    pub fn kind(&self) -> ServiceKind {
        match self {
            Self::BugzillaRestV1(config) => config.kind(),
            Self::Github(config) => config.kind(),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.base(), self.kind())
    }
}

/// Service support
pub enum Service {
    BugzillaRestV1(bugzilla::Service),
    Github(github::Service),
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.base(), self.kind())
    }
}

impl WebService for Service {
    fn base(&self) -> &Url {
        match self {
            Self::BugzillaRestV1(service) => service.base(),
            Self::Github(service) => service.base(),
        }
    }

    fn kind(&self) -> ServiceKind {
        match self {
            Self::BugzillaRestV1(service) => service.kind(),
            Self::Github(service) => service.kind(),
        }
    }

    fn client(&self) -> &reqwest::Client {
        match self {
            Self::BugzillaRestV1(service) => service.client(),
            Self::Github(service) => service.client(),
        }
    }

    fn get_request<S>(&self, id: S, comments: bool, attachments: bool) -> crate::Result<Request>
    where
        S: std::fmt::Display,
    {
        match self {
            Self::BugzillaRestV1(service) => service.get_request(id, comments, attachments),
            Self::Github(service) => service.get_request(id, comments, attachments),
        }
    }

    fn search_request<P: Params>(&self, query: P) -> crate::Result<Request> {
        match self {
            Self::BugzillaRestV1(service) => service.search_request(query),
            Self::Github(service) => service.search_request(query),
        }
    }
}
