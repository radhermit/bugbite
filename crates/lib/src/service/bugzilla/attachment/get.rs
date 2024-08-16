use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    ids: Vec<String>,
    data: bool,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I, S>(service: &'a Service, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service,
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
            .join(&format!("rest/bug/attachment/{id}"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in &self.ids[1..] {
            url.query_pairs_mut().append_pair("attachment_ids", id);
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

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let mut data = data["attachments"].take();

        let mut attachments = vec![];
        for id in self.ids {
            let data = data[&id].take();

            // bugzilla doesn't return errors for nonexistent attachment IDs
            if data.is_null() {
                return Err(Error::InvalidValue(format!("nonexistent attachment: {id}")));
            }

            // bugzilla doesn't return errors for deleted attachments
            if self.data && data["data"].is_null() {
                return Err(Error::InvalidValue(format!(
                    "can't retrieve deleted attachment: {id}"
                )));
            }

            let attachment = serde_json::from_value(data).map_err(|_| {
                Error::InvalidValue(format!("failed deserializing attachment: {id}"))
            })?;

            attachments.push(attachment);
        }

        Ok(attachments)
    }
}
