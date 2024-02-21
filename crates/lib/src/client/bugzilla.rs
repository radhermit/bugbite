use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::bugzilla::{Attachment, Bug, Comment, Event};
use crate::service::bugzilla::attach::CreateAttachment;
use crate::service::bugzilla::{Config, Service};
use crate::time::TimeDelta;
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

    pub async fn attach(&self, attachment: CreateAttachment) -> crate::Result<Vec<u64>> {
        let request = self.service.attach_request(attachment)?;
        request.send(&self.service).await
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

    pub async fn search<Q: Query>(&self, query: Q) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(query)?;
        request.send(&self.service).await
    }
}

#[cfg(test)]
mod tests {
    use crate::service::ServiceKind;
    use crate::test::{TestServer, TESTDATA_PATH};

    use super::*;

    #[tokio::test]
    async fn get() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let client = server
            .client(ServiceKind::Bugzilla)
            .into_bugzilla()
            .unwrap();

        server.respond(200, path.join("get/single-bug.json")).await;
        let bugs = client.get(&[12345], false, false, false).await.unwrap();
        assert_eq!(bugs.len(), 1);
        let bug = &bugs[0];
        assert_eq!(bug.id, 12345);

        server.reset().await;

        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let result = client.get(&[1], false, false, false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let client = server
            .client(ServiceKind::Bugzilla)
            .into_bugzilla()
            .unwrap();

        server.respond(200, path.join("search/ids.json")).await;
        let mut query = client.service().search_query();
        query.insert("summary", "test");
        let bugs = client.search(query).await.unwrap();
        assert_eq!(bugs.len(), 5);
    }
}
