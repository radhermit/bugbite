use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::redmine::Issue;
use crate::service::redmine::{Config, Service};
use crate::traits::{Query, Request, WebService};

#[derive(Debug)]
pub struct Client {
    service: Service,
}

impl Client {
    pub fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        info!("{config}");
        Ok(Self {
            service: Service::new(config, builder)?,
        })
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    /// Return the website URL for an item ID.
    pub fn item_url<S>(&self, id: S) -> String
    where
        S: std::fmt::Display,
    {
        let base = &self.service.config.web_base;
        format!("{base}/issues/{id}")
    }

    pub async fn get<S>(&self, ids: &[S], attachments: bool) -> crate::Result<Vec<Issue>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.get_request(ids, attachments, false, false)?;
        request.send(&self.service).await
    }

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}
