use std::collections::HashSet;
use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

use reqwest::{ClientBuilder, RequestBuilder};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::objects::Ids;
use crate::service::ServiceKind;
use crate::time::TimeDelta;
use crate::traits::{Api, Query, ServiceParams, WebClient, WebService};
use crate::Error;

pub mod attach;
mod attachment;
mod comments;
mod get;
mod history;
pub mod modify;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    pub user: Option<String>,
    pub password: Option<String>,
    pub key: Option<String>,
    cache: ServiceCache,
}

impl Config {
    pub fn new(base: &str) -> crate::Result<Self> {
        let base = base.trim_end_matches('/');
        let base = Url::parse(&format!("{base}/"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {base}: {e}")))?;

        Ok(Self {
            base,
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
        ServiceKind::Bugzilla
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Service: {} -- {}", self.kind(), self.base())
    }
}

#[derive(Debug)]
pub struct Service {
    config: Config,
    client: reqwest::Client,
}

impl Service {
    pub(crate) fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        Ok(Self {
            config,
            client: builder.build()?,
        })
    }

    pub(crate) fn attach_request(
        &self,
        ids: &[NonZeroU64],
        attachments: Vec<attach::CreateAttachment>,
    ) -> crate::Result<attach::AttachRequest> {
        attach::AttachRequest::new(self, ids, attachments)
    }

    pub(crate) fn attachment_request(
        &self,
        ids: &[NonZeroU64],
        data: bool,
    ) -> crate::Result<attachment::AttachmentRequest> {
        attachment::AttachmentRequest::new(self, Ids::object(ids), data)
    }

    pub(crate) fn item_attachment_request(
        &self,
        ids: &[NonZeroU64],
        data: bool,
    ) -> crate::Result<attachment::AttachmentRequest> {
        attachment::AttachmentRequest::new(self, Ids::item(ids), data)
    }

    pub(crate) fn comments_request(
        &self,
        ids: &[NonZeroU64],
        created: Option<&TimeDelta>,
    ) -> crate::Result<comments::CommentsRequest> {
        comments::CommentsRequest::new(self, ids, created)
    }

    pub(crate) fn history_request(
        &self,
        ids: &[NonZeroU64],
        created: Option<&TimeDelta>,
    ) -> crate::Result<history::HistoryRequest> {
        history::HistoryRequest::new(self, ids, created)
    }
}

/// Return a bugzilla error if one is returned in the response data.
macro_rules! return_if_error {
    ($data:expr) => {{
        if $data.get("error").is_some() {
            let code = $data["code"].as_i64().unwrap_or_default();
            let message = if let Some(value) = $data["message"].as_str() {
                value.to_string()
            } else {
                format!("unknown error: {code}")
            };
            return Err(Error::Bugzilla { code, message });
        }
    }};
}

impl<'a> WebClient<'a> for Service {
    type Service = Self;
    type ModifyParams = modify::ModifyParams<'a>;
    type SearchQuery = search::QueryBuilder<'a>;

    fn service(&self) -> &Self::Service {
        self
    }

    fn modify_params(&'a self) -> Self::ModifyParams {
        Self::ModifyParams::new(self.service())
    }

    fn search_query(&'a self) -> Self::SearchQuery {
        Self::SearchQuery::new(self.service())
    }
}

impl<'a> WebService<'a> for Service {
    const API_VERSION: &'static str = "v1";
    type Response = serde_json::Value;
    type GetRequest = get::GetRequest;
    type ModifyRequest = modify::ModifyRequest;
    type SearchRequest = search::SearchRequest;

    fn base(&self) -> &Url {
        self.config.base()
    }

    fn user(&self) -> Option<&str> {
        self.config.user.as_deref()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn inject_auth(
        &self,
        request: RequestBuilder,
        required: bool,
    ) -> crate::Result<RequestBuilder> {
        let config = &self.config;
        if let Some(key) = config.key.as_ref() {
            Ok(request.query(&[("Bugzilla_api_key", key)]))
        } else if let (Some(user), Some(pass)) = (&config.user, &config.password) {
            Ok(request.query(&[("Bugzilla_login", user), ("Bugzilla_password", pass)]))
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
                let data: serde_json::Value = response.json().await?;
                debug!("{data}");
                return_if_error!(&data);
                Ok(data)
            }
            Err(e) => {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    debug!("{data}");
                    return_if_error!(&data);
                }
                Err(e.into())
            }
        }
    }

    fn get_request(
        &self,
        ids: &[NonZeroU64],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Self::GetRequest> {
        get::GetRequest::new(self, ids, attachments, comments, history)
    }

    fn modify_request(
        &self,
        ids: &[NonZeroU64],
        params: Self::ModifyParams,
    ) -> crate::Result<Self::ModifyRequest> {
        modify::ModifyRequest::new(self, ids, params)
    }

    fn search_request<Q: Query>(&self, query: Q) -> crate::Result<Self::SearchRequest> {
        search::SearchRequest::new(self, query)
    }
}

#[derive(
    Display, EnumIter, EnumString, VariantNames, Debug, Default, Eq, PartialEq, Hash, Clone, Copy,
)]
#[strum(serialize_all = "kebab-case")]
pub enum GroupField {
    /// All possible fields
    All,
    /// All default fields
    #[default]
    Default,
    /// All extra fields
    Extra,
    /// All custom fields
    Custom,
}

impl From<GroupField> for FilterField {
    fn from(value: GroupField) -> Self {
        Self::Group(value)
    }
}

impl Api for GroupField {
    type Output = &'static str;
    fn api(&self) -> Self::Output {
        match self {
            Self::All => "_all",
            Self::Default => "_default",
            Self::Extra => "_extra",
            Self::Custom => "_custom",
        }
    }
}

#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum BugField {
    Alias,
    AssignedTo,
    Blocks,
    Cc,
    Component,
    Created,
    Creator,
    Deadline,
    DependsOn,
    Id,
    Keywords,
    Os,
    Platform,
    Priority,
    Product,
    Resolution,
    SeeAlso,
    Severity,
    Status,
    Summary,
    Target,
    Updated,
    Url,
    Version,
    Whiteboard,
}

impl From<BugField> for FilterField {
    fn from(value: BugField) -> Self {
        Self::Bug(value)
    }
}

impl Api for BugField {
    type Output = &'static str;
    fn api(&self) -> Self::Output {
        match self {
            Self::Alias => "alias",
            Self::AssignedTo => "assigned_to",
            Self::Blocks => "blocks",
            Self::Cc => "cc",
            Self::Component => "component",
            Self::Created => "creation_time",
            Self::Creator => "creator",
            Self::Deadline => "deadline",
            Self::DependsOn => "depends_on",
            Self::Id => "id",
            Self::Keywords => "keywords",
            Self::Os => "op_sys",
            Self::Platform => "platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Resolution => "resolution",
            Self::SeeAlso => "see_also",
            Self::Severity => "severity",
            Self::Status => "status",
            Self::Summary => "summary",
            Self::Target => "target_milestone",
            Self::Url => "url",
            Self::Updated => "last_change_time",
            Self::Version => "version",
            Self::Whiteboard => "whiteboard",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum FilterField {
    Bug(BugField),
    Group(GroupField),
}

impl fmt::Display for FilterField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bug(value) => value.fmt(f),
            Self::Group(value) => value.fmt(f),
        }
    }
}

impl FromStr for FilterField {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Ok(value) = BugField::from_str(s) {
            Ok(Self::Bug(value))
        } else if let Ok(value) = GroupField::from_str(s) {
            Ok(Self::Group(value))
        } else {
            Err(Error::InvalidValue(format!("invalid filter field: {s}")))
        }
    }
}

impl Api for FilterField {
    type Output = &'static str;
    fn api(&self) -> Self::Output {
        match self {
            Self::Bug(value) => value.api(),
            Self::Group(value) => value.api(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {
    fields: HashSet<String>,
}
