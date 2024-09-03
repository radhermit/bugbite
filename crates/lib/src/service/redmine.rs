use std::fmt;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::traits::{WebClient, WebService};
use crate::Error;

use super::{ClientBuilder, ServiceKind};

pub mod get;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    #[serde(skip)]
    pub(crate) web_base: Option<Url>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub key: Option<String>,
    pub max_search_results: Option<usize>,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let web_base = if let Some((base, _project)) = base.split_once("/projects/") {
            let url = Url::parse(base)
                .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;
            Some(url)
        } else {
            None
        };

        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            web_base,
            user: None,
            password: None,
            key: None,
            max_search_results: None,
        })
    }

    /// Maximum number of results that can be returned by a search request.
    ///
    /// Fallback to redmine's internal default of 100.
    pub(crate) fn max_search_results(&self) -> usize {
        match self.max_search_results.unwrap_or_default() {
            0 => 100,
            n => n,
        }
    }

    /// Return the base URL for the service.
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// Return the base URL for the service, removing any project subpath if it exists.
    pub fn web_base(&self) -> &Url {
        self.web_base.as_ref().unwrap_or(&self.base)
    }

    /// Return the service variant.
    pub fn kind(&self) -> ServiceKind {
        ServiceKind::Redmine
    }
}

// TODO: remove this once authentication support is added
#[derive(Debug)]
pub struct Service {
    config: Config,
    _cache: ServiceCache,
    client: reqwest::Client,
}

impl Service {
    pub fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            _cache: Default::default(),
            client: builder.build()?,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: fmt::Display>(&self, id: I) -> String {
        let base = self.config.web_base().as_str().trim_end_matches('/');
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
        request: RequestBuilder,
        required: bool,
    ) -> crate::Result<RequestBuilder> {
        let config = &self.config;
        if let Some(key) = config.key.as_ref() {
            Ok(request.header("X-Redmine-API-Key", key))
        } else if let (Some(user), Some(pass)) = (&config.user, &config.password) {
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
                debug!("{data}");
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
                    debug!("{data}");
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

impl<'a> WebClient<'a> for Service {
    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
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
