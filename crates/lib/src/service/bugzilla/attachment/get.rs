use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::objects::{Ids, IdsSlice};
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct AttachmentGetRequest {
    ids: Ids,
    url: Url,
    data: bool,
}

impl AttachmentGetRequest {
    pub(crate) fn new(service: &Service, ids: Ids, data: bool) -> crate::Result<Self> {
        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        let mut url = match ids.as_slice() {
            IdsSlice::Item([id, remaining_ids @ ..]) => {
                let mut url = service.base().join(&format!("rest/bug/{id}/attachment"))?;
                for id in remaining_ids {
                    url.query_pairs_mut().append_pair("ids", id.as_str());
                }
                url
            }
            IdsSlice::Object([id, remaining_ids @ ..]) => {
                let mut url = service.base().join(&format!("rest/bug/attachment/{id}"))?;
                for id in remaining_ids {
                    url.query_pairs_mut()
                        .append_pair("attachment_ids", id.as_str());
                }
                url
            }
            _ => return Err(Error::InvalidRequest("no IDs specified".to_string())),
        };

        if !data {
            url.query_pairs_mut().append_pair("exclude_fields", "data");
        }

        Ok(Self { ids, url, data })
    }
}

impl Request for AttachmentGetRequest {
    type Output = Vec<Vec<Attachment>>;
    type Service = Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.url).auth_optional(service)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        match self.ids {
            Ids::Item(_) => {
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
                            "invalid service response to get request".to_string(),
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
            Ids::Object(ids) => {
                let mut data = data["attachments"].take();
                let mut attachments = vec![];
                for id in ids {
                    let data = data[&id].take();
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
                Ok(vec![attachments])
            }
        }
    }
}
