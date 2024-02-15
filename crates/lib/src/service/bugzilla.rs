use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::service::ServiceKind;
use crate::traits::{Params, WebService};
use crate::Error;

mod get;
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
    client: reqwest::Client,
}

impl WebService for Service {
    type Response = serde_json::Value;
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

    fn get_request<S>(
        &self,
        ids: &[S],
        comments: bool,
        attachments: bool,
    ) -> crate::Result<Self::GetRequest>
    where
        S: std::fmt::Display,
    {
        get::GetRequest::new(self, ids, comments, attachments)
    }

    fn search_request<P: Params>(&self, query: P) -> crate::Result<Self::SearchRequest> {
        search::SearchRequest::new(self, query)
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ServiceCache {
    fields: HashSet<String>,
}
