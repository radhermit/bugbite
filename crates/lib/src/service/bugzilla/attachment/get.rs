use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

pub struct Request {
    ids: Vec<String>,
    url: Url,
    data: bool,
}

impl Request {
    pub(crate) fn new<I, S>(service: &Service, ids: I) -> crate::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        let ids: Vec<_> = ids.into_iter().map(|s| s.to_string()).collect();
        let id = ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = service
            .config
            .base
            .join(&format!("rest/bug/attachment/{id}"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in &ids[1..] {
            url.query_pairs_mut().append_pair("attachment_ids", id);
        }

        Ok(Self {
            ids,
            url,
            data: true,
        })
    }

    pub fn data(mut self, fetch: bool) -> Self {
        if !fetch {
            self.url
                .query_pairs_mut()
                .append_pair("exclude_fields", "data");
        }

        self.data = fetch;
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<Attachment>;
    type Service = Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client.get(self.url).auth_optional(service)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
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
