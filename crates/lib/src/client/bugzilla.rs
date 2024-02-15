use crate::objects::bugzilla::Bug;
use crate::service::bugzilla::Service;
use crate::traits::{Params, Request, WebService};

#[derive(Debug)]
pub struct Client {
    service: Service,
}

impl Client {
    pub fn new(service: Service) -> Self {
        Self { service }
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    pub async fn get<S>(
        &self,
        ids: &[S],
        comments: bool,
        attachments: bool,
    ) -> crate::Result<Vec<Bug>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.get_request(ids, comments, attachments)?;
        request.send(&self.service).await
    }

    pub async fn search<Q: Params>(&self, query: Q) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}
