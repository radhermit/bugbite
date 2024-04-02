use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::redmine::Issue;
use crate::service::redmine::{search, Config, Service};
use crate::traits::{Request, WebService};

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
    pub fn search_url(&self, params: search::Parameters) -> crate::Result<String> {
        let base = self.service.base().as_str().trim_end_matches('/');
        let params = params.encode(&self.service)?;
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
        request.send(&self.service).await
    }

    pub async fn search(&self, params: search::Parameters) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(params)?;
        request.send(&self.service).await
    }
}
