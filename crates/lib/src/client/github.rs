use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::github::Issue;
use crate::service::github::{search, Config, Service};
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

    pub async fn get(&self, ids: &[u64]) -> crate::Result<Vec<Issue>> {
        let request = self.service.get_request(ids, false, false, false)?;
        request.send(&self.service).await
    }

    pub async fn search(&self, params: search::Parameters) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(params)?;
        request.send(&self.service).await
    }
}
