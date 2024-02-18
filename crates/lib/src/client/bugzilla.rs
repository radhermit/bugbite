use reqwest::ClientBuilder;

use crate::objects::bugzilla::{Attachment, Bug, Comment, Event};
use crate::service::bugzilla::{Config, Service};
use crate::time::TimeDelta;
use crate::traits::{Params, Request, WebService};

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

    pub async fn attachments<S>(&self, ids: &[S], data: bool) -> crate::Result<Vec<Vec<Attachment>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.attachments_request(ids, data)?;
        request.send(&self.service).await
    }

    pub async fn item_attachments<S>(
        &self,
        ids: &[S],
        data: bool,
    ) -> crate::Result<Vec<Vec<Attachment>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.item_attachments_request(ids, data)?;
        request.send(&self.service).await
    }

    pub async fn comments<S>(
        &self,
        ids: &[S],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Vec<Vec<Comment>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.comments_request(ids, created)?;
        request.send(&self.service).await
    }

    pub async fn get<S>(
        &self,
        ids: &[S],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Vec<Bug>>
    where
        S: std::fmt::Display,
    {
        let request = self
            .service
            .get_request(ids, attachments, comments, history)?;
        request.send(&self.service).await
    }

    pub async fn history<S>(
        &self,
        ids: &[S],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Vec<Vec<Event>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.history_request(ids, created)?;
        request.send(&self.service).await
    }

    pub async fn search<Q: Params>(&self, query: Q) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}
