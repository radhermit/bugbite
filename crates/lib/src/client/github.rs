use reqwest::ClientBuilder;
use tracing::info;

use crate::service::github::{Config, Service};

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
}
