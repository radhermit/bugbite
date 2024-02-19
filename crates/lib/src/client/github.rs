use reqwest::ClientBuilder;

use crate::objects::github::Issue;
use crate::service::github::{Config, Service};
use crate::traits::{Query, Request, WebService};

#[derive(Debug)]
pub struct Client {
    service: Service,
}

impl Client {
    pub(crate) fn new(config: Config, builder: ClientBuilder) -> crate::Result<Self> {
        let client = builder.build()?;
        Ok(Self {
            service: config.service(client),
        })
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    pub async fn get<S>(
        &self,
        ids: &[S],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Vec<Issue>>
    where
        S: std::fmt::Display,
    {
        let request = self
            .service
            .get_request(ids, attachments, comments, history)?;
        request.send(&self.service).await
    }

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Issue>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}
