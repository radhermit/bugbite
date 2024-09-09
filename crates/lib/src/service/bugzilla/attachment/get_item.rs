use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<String>,
    pub data: bool,
}

impl Request {
    pub(crate) fn new<I, S>(service: &Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service: service.clone(),
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            data: true,
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = self
            .service
            .config
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

    pub fn data(mut self, fetch: bool) -> Self {
        self.data = fetch;
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<Vec<Attachment>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let Value::Object(data) = data else {
            panic!("invalid bugzilla attachment response");
        };

        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut attachments = vec![];
        for (id, values) in data {
            let Value::Array(data) = values else {
                return Err(Error::InvalidResponse("attachment get request".to_string()));
            };

            let mut bug_attachments = vec![];
            for attachment in data {
                // skip deserializing deleted attachments when retrieving data
                if !self.data || !attachment["data"].is_null() {
                    let attachment = serde_json::from_value(attachment).map_err(|_| {
                        Error::InvalidResponse(format!("invalid attachment for bug {id}"))
                    })?;
                    bug_attachments.push(attachment);
                }
            }

            attachments.push(bug_attachments);
        }

        Ok(attachments)
    }
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent bug
        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let err = service.attachment_get_item([1]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::Bugzilla { code: 101, .. }),
            "unmatched error: {err:?}"
        );

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
