use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::bugzilla::{Attachment, Bug, Comment, Event};
use crate::service::bugzilla::{
    attach::CreateAttachment,
    comment::CommentParams,
    create,
    modify::{self, BugChange},
    search, {Config, Service},
};
use crate::time::TimeDelta;
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
        Ok(format!("{base}/buglist.cgi?{params}"))
    }

    pub async fn attach<S>(
        &self,
        ids: &[S],
        attachments: Vec<CreateAttachment>,
    ) -> crate::Result<Vec<Vec<u64>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.attach_request(ids, attachments)?;
        request.send(&self.service).await
    }

    pub async fn attachment<S>(
        &self,
        ids: &[S],
        bugs: bool,
        data: bool,
    ) -> crate::Result<Vec<Vec<Attachment>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.attachment_request(ids, bugs, data)?;
        request.send(&self.service).await
    }

    pub async fn comment<S>(
        &self,
        ids: &[S],
        params: Option<CommentParams>,
    ) -> crate::Result<Vec<Vec<Comment>>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.comment_request(ids, params)?;
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

    pub async fn create(&self, params: create::Parameters) -> crate::Result<u64> {
        let request = self.service.create_request(params)?;
        request.send(&self.service).await
    }

    pub async fn modify<S>(
        &self,
        ids: &[S],
        params: modify::Parameters,
    ) -> crate::Result<Vec<BugChange>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.modify_request(ids, params)?;
        request.send(&self.service).await
    }

    pub async fn search(&self, params: search::Parameters) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(params)?;
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
        let query = search::Parameters::new().summary(["test"]);
        let bugs = client.search(query).await.unwrap();
        assert_eq!(bugs.len(), 5);
    }
}
