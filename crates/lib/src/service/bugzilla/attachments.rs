use itertools::Itertools;
use tracing::debug;
use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::objects::{Ids, IdsSlice};
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct AttachmentsRequest {
    ids: Ids,
    req: reqwest::Request,
}

impl AttachmentsRequest {
    pub(crate) fn new(service: &super::Service, ids: Ids, data: bool) -> crate::Result<Self> {
        let mut params = vec![];
        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        let mut url = match ids.as_slice() {
            IdsSlice::Item([id, ids @ ..]) => {
                if !ids.is_empty() {
                    params.push(("ids".to_string(), ids.iter().join(",")));
                }
                service.base().join(&format!("/rest/bug/{id}/attachment"))?
            }
            IdsSlice::Object([id, ids @ ..]) => {
                if !ids.is_empty() {
                    params.push(("attachment_ids".to_string(), ids.iter().join(",")));
                }
                service.base().join(&format!("/rest/bug/attachment/{id}"))?
            }
            _ => {
                return Err(Error::InvalidValue(
                    "bug or attachment IDs not specified".to_string(),
                ))
            }
        };

        if !data {
            params.push(("exclude_fields".to_string(), "data".to_string()));
        }

        if !params.is_empty() {
            url = Url::parse_with_params(url.as_str(), params)?;
        }

        Ok(AttachmentsRequest {
            ids,
            req: service.client().get(url).build()?,
        })
    }
}

impl Request for AttachmentsRequest {
    type Output = Vec<Vec<Attachment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
        let mut data = service.parse_response(response).await?;
        match self.ids {
            Ids::Item(ids) => {
                debug!("attachments request data: {data}");
                let mut attachments = vec![];
                let mut data = data["bugs"].take();
                for id in ids {
                    let data = data[&id].take();
                    attachments.push(serde_json::from_value::<Vec<Attachment>>(data)?);
                }
                Ok(attachments)
            }
            Ids::Object(ids) => {
                debug!("attachments request data: {data}");
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
