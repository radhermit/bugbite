use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::bugzilla::{Attachment, Bug, Comment, Event};
use crate::service::bugzilla::attach::CreateAttachment;
use crate::service::bugzilla::comment::CommentParams;
use crate::service::bugzilla::create::CreateParams;
use crate::service::bugzilla::modify::{BugChange, ModifyParams};
use crate::service::bugzilla::search::QueryBuilder;
use crate::service::bugzilla::{Config, Service};
use crate::time::TimeDelta;
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
        request.send().await
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
        request.send().await
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
        request.send().await
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
        request.send().await
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
        request.send().await
    }

    pub async fn create<'a>(&'a self, params: CreateParams<'a>) -> crate::Result<u64> {
        let request = self.service.create_request(params)?;
        request.send().await
    }

    pub async fn modify<'a, S>(
        &'a self,
        ids: &[S],
        params: ModifyParams<'a>,
    ) -> crate::Result<Vec<BugChange>>
    where
        S: std::fmt::Display,
    {
        let request = self.service.modify_request(ids, params)?;
        request.send().await
    }

    pub async fn search<'a>(&'a self, query: QueryBuilder<'a>) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(query)?;
        request.send().await
    }
}

#[cfg(test)]
mod tests {
    use crate::service::ServiceKind;
    use crate::test::{TestServer, TESTDATA_PATH};
    use crate::traits::WebClient;

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
        query.summary(["test"]);
        let bugs = client.search(query).await.unwrap();
        assert_eq!(bugs.len(), 5);
    }
}
