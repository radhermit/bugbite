use std::fmt;

use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use strum::{AsRefStr, Display, EnumIter, EnumString, VariantNames};
use url::Url;

pub mod bugzilla;
pub mod github;

use crate::traits::WebService;
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
    /// API docs: https://bugzilla.readthedocs.io/en/latest/api/
    #[default]
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
    pub fn base(&self) -> &Url {
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

#[derive(EnumAsInner, Debug)]
pub enum Service {
    Bugzilla(bugzilla::Service),
    Github(github::Service),
}

impl Service {
    pub fn base(&self) -> &Url {
        match self {
            Self::Bugzilla(service) => service.base(),
            Self::Github(service) => service.base(),
        }
    }

    pub fn kind(&self) -> ServiceKind {
        match self {
            Self::Bugzilla(service) => service.kind(),
            Self::Github(service) => service.kind(),
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.base(), self.kind())
    }
}
