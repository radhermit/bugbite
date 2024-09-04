use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    pub ids: Vec<u64>,
    pub data: bool,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I>(service: &'a Service, ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        Self {
            service,
            ids: ids.into_iter().collect(),
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
            .join(&format!("rest/bug/attachment/{id}"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in self.ids[1..].iter().map(|x| x.to_string()) {
            url.query_pairs_mut().append_pair("attachment_ids", &id);
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

impl RequestSend for Request<'_> {
    type Output = Vec<Attachment>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let mut data = data["attachments"].take();

        let mut attachments = vec![];
        for id in self.ids.iter().map(|x| x.to_string()) {
            let data = data[&id].take();

            // bugzilla doesn't return errors for nonexistent attachment IDs
            if data.is_null() {
                return Err(Error::InvalidValue(format!("nonexistent attachment: {id}")));
            }

            // bugzilla doesn't return errors for deleted attachments
            if self.data && data["data"].is_null() {
                return Err(Error::InvalidValue(format!("deleted attachment: {id}")));
            }

            let attachment = serde_json::from_value(data).map_err(|_| {
                Error::InvalidResponse(format!("failed deserializing attachment: {id}"))
            })?;

            attachments.push(attachment);
        }

        Ok(attachments)
    }
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Config::new(server.uri()).unwrap().service().unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.attachment_get(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent
        server
            .respond(200, path.join("attachment/get/nonexistent.json"))
            .await;
        let err = service.attachment_get([1]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidValue(_)));
        assert_err_re!(err, "nonexistent attachment: 1");

        server.reset().await;

        // deleted
        server
            .respond(200, path.join("attachment/get/deleted.json"))
            .await;
        let err = service.attachment_get([21]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidValue(_)));
        assert_err_re!(err, "deleted attachment: 21");

        server.reset().await;

        // invalid response
        server
            .respond(200, path.join("attachment/get/invalid.json"))
            .await;
        let err = service.attachment_get([123]).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidResponse(_)));
        assert_err_re!(err, "failed deserializing attachment: 123");

        server.reset().await;

        // single without data
        server
            .respond(200, path.join("attachment/get/single-without-data.json"))
            .await;
        let attachment = &service
            .attachment_get([123])
            .data(false)
            .send()
            .await
            .unwrap()[0];
        assert!(attachment.is_empty());

        server.reset().await;

        // single with plain text data
        server
            .respond(200, path.join("attachment/get/single-plain-text.json"))
            .await;
        let attachment = &service.attachment_get([123]).send().await.unwrap()[0];
        assert_eq!(attachment.id, 123);
        assert_eq!(attachment.bug_id, 321);
        assert_eq!(attachment.file_name, "test.txt");
        assert_eq!(attachment.summary, "test.txt");
        assert_eq!(attachment.size, 8);
        assert_eq!(attachment.creator, "person");
        assert_eq!(attachment.content_type, "text/plain");
        assert!(!attachment.is_private);
        assert!(!attachment.is_obsolete);
        assert!(!attachment.is_patch);
        assert_eq!(attachment.created.to_string(), "2024-02-19 08:35:02 UTC");
        assert_eq!(attachment.updated.to_string(), "2024-02-19 08:35:02 UTC");
        assert!(attachment.flags.is_empty());
        assert_eq!(String::from_utf8_lossy(attachment.as_ref()), "bugbite\n");

        server.reset().await;

        // multiple with plain text data
        server
            .respond(200, path.join("attachment/get/multiple-plain-text.json"))
            .await;
        let ids = [123, 124];
        let attachments = &service.attachment_get(ids).send().await.unwrap();
        assert_ordered_eq!(attachments.iter().map(|x| x.id), ids);
    }
}
