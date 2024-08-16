use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

pub struct Request<'a> {
    service: &'a Service,
    url: Url,
    data: bool,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I, S>(service: &'a Service, ids: I) -> crate::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        let mut ids = ids.into_iter().map(|s| s.to_string());
        let id = ids
            .next()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = service
            .config
            .base
            .join(&format!("rest/bug/{id}/attachment"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in ids {
            url.query_pairs_mut().append_pair("ids", &id);
        }

        Ok(Self {
            service,
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

impl RequestSend for Request<'_> {
    type Output = Vec<Vec<Attachment>>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url)
            .auth_optional(self.service)?;
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
                return Err(Error::InvalidValue(
                    "invalid service response to attachment get request".to_string(),
                ));
            };

            let mut bug_attachments = vec![];
            for attachment in data {
                // skip deserializing deleted attachments when retrieving data
                if !self.data || !attachment["data"].is_null() {
                    let attachment = serde_json::from_value(attachment).map_err(|_| {
                        Error::InvalidValue(format!("invalid attachment for bug {id}"))
                    })?;
                    bug_attachments.push(attachment);
                }
            }

            attachments.push(bug_attachments);
        }

        Ok(attachments)
    }
}
