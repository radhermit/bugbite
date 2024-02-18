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

#[cfg(test)]
mod tests {
    use std::fs;

    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::client::Client;
    use crate::service::ServiceKind;
    use crate::test::build_path;

    #[tokio::test]
    async fn get() {
        let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");
        let json = fs::read_to_string(path.join("bugzilla/get/single-bug.json")).unwrap();

        let mock_server = MockServer::start().await;
        let template = ResponseTemplate::new(200).set_body_raw(json.as_bytes(), "application/json");
        Mock::given(any())
            .respond_with(template)
            .mount(&mock_server)
            .await;

        let service = ServiceKind::BugzillaRestV1
            .create(&mock_server.uri())
            .unwrap();
        let client = Client::builder()
            .build(service)
            .unwrap()
            .into_bugzilla()
            .unwrap();

        let bugs = client.get(&[12345], false, false, false).await.unwrap();
        assert_eq!(bugs.len(), 1);
        let bug = &bugs[0];
        assert_eq!(bug.id(), 12345);
    }
}
