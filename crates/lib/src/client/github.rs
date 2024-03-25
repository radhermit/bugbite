use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::github::Issue;
use crate::service::github::{Config, Service};
use crate::traits::{Query, Request, WebService};

#[derive(Debug)]
pub struct Client {
    service: Service,
}

impl Client {
    pub fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        let service = Service::new(config, builder)?;
        info!("Service: {service}");
        Ok(Self { service })
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    pub async fn get(
        &self,
        ids: &[u64],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Vec<Issue>> {
        let request = self
            .service
            .get_request(ids, attachments, comments, history)?;
        request.send().await
    }

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(query)?;
        request.send().await
    }
}
