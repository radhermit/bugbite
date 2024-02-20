use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::{debug, trace};
use url::Url;

use crate::objects::Ids;
use crate::service::ServiceKind;
use crate::time::TimeDelta;
use crate::traits::{Api, Query, WebService};
use crate::Error;

mod attachments;
mod comments;
mod get;
mod history;
pub mod search;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    base: Url,
    cache: ServiceCache,
}

impl Config {
    pub(super) fn new(base: Url) -> Self {
        Self {
            base,
            cache: Default::default(),
        }
    }

    pub(crate) fn service(self, client: reqwest::Client) -> Service {
        Service {
            config: self,
            user: None,
            password: None,
            api_key: None,
            client,
        }
    }

    pub fn base(&self) -> &Url {
        &self.base
    }

    pub fn kind(&self) -> ServiceKind {
        ServiceKind::BugzillaRestV1
    }
}

// TODO: remove this once authentication support is added
#[allow(dead_code)]
#[derive(Debug)]
pub struct Service {
    config: Config,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl Service {
    pub(crate) fn attachments_request<S>(
        &self,
        ids: &[S],
        data: bool,
    ) -> crate::Result<attachments::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        attachments::AttachmentsRequest::new(self, Ids::object(ids), data)
    }

    pub(crate) fn item_attachments_request<S>(
        &self,
        ids: &[S],
        data: bool,
    ) -> crate::Result<attachments::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        attachments::AttachmentsRequest::new(self, Ids::item(ids), data)
    }

    pub(crate) fn comments_request<S>(
        &self,
        ids: &[S],
        created: Option<&TimeDelta>,
    ) -> crate::Result<comments::CommentsRequest>
    where
        S: std::fmt::Display,
    {
        comments::CommentsRequest::new(self, ids, created)
    }

    pub(crate) fn history_request<S>(
        &self,
        ids: &[S],
        created: Option<&TimeDelta>,
    ) -> crate::Result<history::HistoryRequest>
    where
        S: std::fmt::Display,
    {
        history::HistoryRequest::new(self, ids, created)
    }
}

impl WebService for Service {
    const API_VERSION: &'static str = "v1";
    type Response = serde_json::Value;
    type GetRequest = get::GetRequest;
    type SearchRequest = search::SearchRequest;
    type SearchQuery = search::QueryBuilder;

    fn base(&self) -> &Url {
        self.config.base()
    }

    fn kind(&self) -> ServiceKind {
        self.config.kind()
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    async fn parse_response(&self, response: reqwest::Response) -> crate::Result<Self::Response> {
        trace!("{response:?}");
        let data: serde_json::Value = response.json().await?;
        debug!("{data}");
        if data.get("error").is_some() {
            let code = data["code"].as_i64().unwrap();
            let message = data["message"].as_str().unwrap().to_string();
            Err(Error::Bugzilla { code, message })
        } else {
            Ok(data)
        }
    }

    fn get_request<S>(
        &self,
        ids: &[S],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Self::GetRequest>
    where
        S: std::fmt::Display,
    {
        get::GetRequest::new(self, ids, attachments, comments, history)
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
    Id,
    AssignedTo,
    Summary,
    Reporter,
    Status,
    Resolution,
    Whiteboard,
    Product,
    Component,
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
            Self::Id => "id",
            Self::AssignedTo => "assigned_to",
            Self::Summary => "summary",
            Self::Reporter => "creator",
            Self::Status => "status",
            Self::Resolution => "resolution",
            Self::Whiteboard => "whiteboard",
            Self::Product => "product",
            Self::Component => "component",
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
