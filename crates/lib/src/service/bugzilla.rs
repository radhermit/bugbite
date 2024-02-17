use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::service::ServiceKind;
use crate::time::TimeDelta;
use crate::traits::{Params, WebService};
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

#[derive(Debug)]
pub struct Service {
    config: Config,
    user: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl Service {
    pub(crate) fn comments_request<S>(
        &self,
        ids: &[S],
        created: Option<TimeDelta>,
    ) -> crate::Result<comments::CommentsRequest>
    where
        S: std::fmt::Display,
    {
        comments::CommentsRequest::new(self, ids, created)
    }

    pub(crate) fn history_request<S>(
        &self,
        ids: &[S],
        created: Option<TimeDelta>,
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
    type AttachmentsRequest = attachments::AttachmentsRequest;
    type GetRequest = get::GetRequest;
    type SearchRequest = search::SearchRequest;

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
        let data = response.text().await?;
        let data: serde_json::Value = data.parse()?;
        if data.get("error").is_some() {
            let code = data["code"].as_i64().unwrap();
            let message = data["message"].as_str().unwrap().to_string();
            Err(Error::Bugzilla { code, message })
        } else {
            Ok(data)
        }
    }

    fn attachments_request<S>(
        &self,
        ids: &[S],
        data: bool,
    ) -> crate::Result<Self::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        attachments::AttachmentsRequest::builder()
            .attachment_ids(ids)
            .data(data)
            .build(self)
    }

    fn item_attachments_request<S>(
        &self,
        ids: &[S],
        data: bool,
    ) -> crate::Result<Self::AttachmentsRequest>
    where
        S: std::fmt::Display,
    {
        attachments::AttachmentsRequest::builder()
            .bug_ids(ids)
            .data(data)
            .build(self)
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

    fn search_request<P: Params>(&self, query: P) -> crate::Result<Self::SearchRequest> {
        search::SearchRequest::new(self, query)
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {
    fields: HashSet<String>,
}
