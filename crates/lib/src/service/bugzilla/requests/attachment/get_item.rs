use indexmap::IndexMap;
use serde::Deserialize;
use url::Url;

use crate::Error;
use crate::service::bugzilla::Bugzilla;
use crate::service::bugzilla::objects::Attachment;
use crate::traits::{InjectAuth, ParseResponse, RequestSend};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,

    /// Bug IDs or aliases to fetch attachments from.
    pub ids: Vec<String>,

    /// Include attachment data.
    pub data: bool,

    /// Include obsolete attachments (skipped by default).
    pub obsolete: bool,
}

impl Request {
    /// Create a new request.
    pub(crate) fn new<I, S>(service: Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service,
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            data: true,
            obsolete: false,
        }
    }

    /// Generate the URL for the request.
    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = self
            .service
            .config()
            .base
            .join(&format!("rest/bug/{id}/attachment"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in &self.ids[1..] {
            url.query_pairs_mut().append_pair("ids", id);
        }

        if !self.data {
            url.query_pairs_mut().append_pair("exclude_fields", "data");
        }

        Ok(url)
    }

    /// Include attachment data.
    pub fn data(&mut self, status: bool) -> &mut Self {
        self.data = status;
        self
    }

    /// Filter obsolete attachments.
    pub fn obsolete(&mut self, status: bool) -> &mut Self {
        self.obsolete = status;
        self
    }
}

/// Bugzilla REST API response to an attachment get request.
///
/// https://bugzilla.readthedocs.io/en/latest/api/core/v1/attachment.html#get-attachment
#[derive(Deserialize, Debug)]
struct Response {
    bugs: IndexMap<u64, Vec<Attachment>>,
    #[serde(rename = "attachments")]
    _attachments: IndexMap<u64, Attachment>,
}

impl RequestSend for Request {
    type Output = Vec<Vec<Attachment>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let data: Response = self.service.parse_response(response).await?;

        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut attachments = vec![];
        for (_, mut bug_attachments) in data.bugs {
            bug_attachments.retain(|x| {
                // skip deleted attachments when retrieving data
                (!self.data || !x.is_deleted()) &&
                // conditionally skip obsolete attachments
                (self.obsolete || !x.is_obsolete)
            });

            attachments.push(bug_attachments);
        }

        Ok(attachments)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.attachment_get_item(ids).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no IDs specified");

        // nonexistent bug
        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let err = service.attachment_get_item([1]).send().await.unwrap_err();
        assert_matches!(err, Error::Service(_));
        assert_err_re!(err, "bugzilla: Bug #1 does not exist.");

        server.reset().await;

        // bug with no attachments
        server
            .respond(
                200,
                path.join("attachment/get/bug-with-no-attachments.json"),
            )
            .await;
        let attachments = &service.attachment_get_item([12345]).send().await.unwrap()[0];
        assert!(attachments.is_empty());

        server.reset().await;

        // bugs with no attachments
        server
            .respond(
                200,
                path.join("attachment/get/bug-with-no-attachments.json"),
            )
            .await;
        let attachments = &service
            .attachment_get_item([12345, 23456, 34567])
            .send()
            .await
            .unwrap();
        assert!(attachments.iter().all(|x| x.is_empty()));
    }
}
