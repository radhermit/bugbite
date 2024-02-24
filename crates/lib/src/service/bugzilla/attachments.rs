use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::objects::{Ids, IdsSlice};
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct AttachmentsRequest {
    ids: Ids,
    url: Url,
}

impl AttachmentsRequest {
    pub(crate) fn new(service: &super::Service, ids: Ids, data: bool) -> crate::Result<Self> {
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

        Ok(AttachmentsRequest { ids, url })
    }
}

impl Request for AttachmentsRequest {
    type Output = Vec<Vec<Attachment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().get(self.url).send().await?;
        let mut data = service.parse_response(response).await?;
        match self.ids {
            Ids::Item(ids) => {
                let mut attachments = vec![];
                let mut data = data["bugs"].take();
                for id in ids {
                    let data = data[&id].take();
                    attachments.push(serde_json::from_value::<Vec<Attachment>>(data)?);
                }
                Ok(attachments)
            }
            Ids::Object(ids) => {
                let mut data = data["attachments"].take();
                let mut attachments = vec![];
                for id in ids {
                    let data = data[&id].take();
                    let attachment = serde_json::from_value(data)
                        .map_err(|_| Error::InvalidValue(format!("unknown attachment ID: {id}")))?;
                    attachments.push(attachment);
                }
                Ok(vec![attachments])
            }
        }
    }
}
