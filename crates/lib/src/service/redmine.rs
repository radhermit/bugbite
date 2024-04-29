use std::fmt;

use reqwest::{ClientBuilder, RequestBuilder};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::traits::{NullRequest, WebService};
use crate::Error;

use super::ServiceKind;

mod get;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    pub(crate) web_base: Url,
    pub user: Option<String>,
    pub password: Option<String>,
    pub key: Option<String>,
    cache: ServiceCache,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let Some((web_base, _project)) = base.split_once("/projects/") else {
            return Err(Error::InvalidValue(format!("invalid project base: {base}")));
        };

        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;
        let web_base = Url::parse(web_base)
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
            web_base,
            user: None,
            password: None,
            key: None,
            cache: Default::default(),
        })
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::Redmine
    }
}

// TODO: remove this once authentication support is added
#[derive(Debug)]
pub struct Service {
    pub(crate) config: Config,
    client: reqwest::Client,
}

impl Service {
    pub(crate) fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            client: builder.build()?,
        })
    }

    /// Return the website URL for an item ID.
    pub fn item_url<I: std::fmt::Display>(&self, id: I) -> String {
        let base = self.config.web_base.as_str().trim_end_matches('/');
        format!("{base}/issues/{id}")
    }

    /// Return the website URL for a query.
    pub fn search_url(&self, params: search::Parameters) -> crate::Result<String> {
        let base = self.base().as_str().trim_end_matches('/');
        let params = params.encode(self)?;
        Ok(format!("{base}/issues?set_filter=1&{params}"))
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
    type GetRequest = get::GetRequest;
    type CreateRequest = NullRequest;
    type CreateParams = ();
    type UpdateRequest = NullRequest;
    type UpdateParams = ();
    type SearchRequest = search::SearchRequest;
    type SearchParams = search::Parameters;

    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

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

    fn get_request<S>(
        &self,
        ids: &[S],
        attachments: bool,
        comments: bool,
        _history: bool,
    ) -> crate::Result<Self::GetRequest>
    where
        S: std::fmt::Display,
    {
        get::GetRequest::new(self, ids, attachments, comments)
    }

    fn search_request(&self, params: Self::SearchParams) -> crate::Result<Self::SearchRequest> {
        search::SearchRequest::new(self, params)
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
