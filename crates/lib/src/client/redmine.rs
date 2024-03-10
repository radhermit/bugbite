use std::num::NonZeroU64;

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
    pub fn item_url(&self, id: NonZeroU64) -> String {
        let base = self.service.config.web_base.as_str().trim_end_matches('/');
        format!("{base}/issues/{id}")
    }

    pub async fn get(
        &self,
        ids: &[NonZeroU64],
        attachments: bool,
        comments: bool,
    ) -> crate::Result<Vec<Issue>> {
        let request = self
            .service
            .get_request(ids, attachments, comments, false)?;
        request.send(&self.service).await
    }

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}
