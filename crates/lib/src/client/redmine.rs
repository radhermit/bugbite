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
        let service = Service::new(config, builder)?;
        info!("Service: {service}");
        Ok(Self { service })
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    /// Return the website URL for a query.
    pub fn search_url<Q: Query>(&self, mut query: Q) -> crate::Result<String> {
        let base = self.service.base().as_str().trim_end_matches('/');
        let params = query.params()?;
        Ok(format!("{base}/issues?set_filter=1&{params}"))
    }

    pub async fn get<S>(
        &self,
        ids: &[S],
        attachments: bool,
        comments: bool,
    ) -> crate::Result<Vec<Issue>>
    where
        S: std::fmt::Display,
    {
        let request = self
            .service
            .get_request(ids, attachments, comments, false)?;
        request.send().await
    }

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(query)?;
        request.send().await
    }
}
