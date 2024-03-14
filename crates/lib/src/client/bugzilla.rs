use std::num::NonZeroU64;

use reqwest::ClientBuilder;
use tracing::info;

use crate::objects::bugzilla::{Attachment, Bug, Comment, Event};
use crate::service::bugzilla::attach::CreateAttachment;
use crate::service::bugzilla::modify::ModifyParams;
use crate::service::bugzilla::{Config, Service};
use crate::time::TimeDelta;
use crate::traits::{Query, Request, WebService};
use crate::Error;

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

    /// Return the website URL for an item ID.
    pub fn item_url<I: Into<u64> + std::fmt::Display>(&self, id: I) -> String {
        let base = self.service.base().as_str().trim_end_matches('/');
        format!("{base}/show_bug.cgi?id={id}")
    }

    pub async fn attach(
        &self,
        ids: &[NonZeroU64],
        attachments: Vec<CreateAttachment>,
    ) -> crate::Result<Vec<Vec<NonZeroU64>>> {
        let request = self.service.attach_request(ids, attachments)?;
        request.send(&self.service).await
    }

    pub async fn attachments(
        &self,
        ids: &[NonZeroU64],
        data: bool,
    ) -> crate::Result<Vec<Vec<Attachment>>> {
        let request = self.service.attachments_request(ids, data)?;
        request.send(&self.service).await
    }

    pub async fn item_attachments(
        &self,
        ids: &[NonZeroU64],
        data: bool,
    ) -> crate::Result<Vec<Vec<Attachment>>> {
        let request = self.service.item_attachments_request(ids, data)?;
        request.send(&self.service).await
    }

    pub async fn comments(
        &self,
        ids: &[NonZeroU64],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Vec<Vec<Comment>>> {
        let request = self.service.comments_request(ids, created)?;
        request.send(&self.service).await
    }

    pub async fn get<N>(
        &self,
        ids: &[N],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Vec<Bug>>
    where
        N: TryInto<NonZeroU64> + Copy,
        <N as TryInto<NonZeroU64>>::Error: std::fmt::Display,
    {
        // TODO: move ID conversion support to a macro
        let ids = ids
            .iter()
            .copied()
            .map(|x| x.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::InvalidValue(format!("invalid ID: {e}")))?;
        let request = self
            .service
            .get_request(&ids, attachments, comments, history)?;
        request.send(&self.service).await
    }

    pub async fn history(
        &self,
        ids: &[NonZeroU64],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Vec<Vec<Event>>> {
        let request = self.service.history_request(ids, created)?;
        request.send(&self.service).await
    }

    pub async fn modify<'a>(
        &'a self,
        ids: &[NonZeroU64],
        params: ModifyParams<'a>,
    ) -> crate::Result<()> {
        let request = self.service.modify_request(ids, params)?;
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
        query.insert("summary", "test");
        let bugs = client.search(query).await.unwrap();
        assert_eq!(bugs.len(), 5);
    }
}
