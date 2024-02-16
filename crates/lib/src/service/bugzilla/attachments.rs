use itertools::Itertools;
use tracing::debug;
use url::Url;

use crate::objects::bugzilla::Attachment;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug, Default)]
pub(crate) struct AttachmentsRequestBuilder {
    bug_ids: Option<Vec<String>>,
    attachment_ids: Option<Vec<String>>,
    data: bool,
    params: Vec<(String, String)>,
}

impl AttachmentsRequestBuilder {
    pub(crate) fn bug_ids<S: std::fmt::Display>(mut self, ids: &[S]) -> Self {
        self.bug_ids = Some(ids.iter().map(|s| s.to_string()).collect());
        self
    }

    pub(crate) fn attachment_ids<S: std::fmt::Display>(mut self, ids: &[S]) -> Self {
        self.attachment_ids = Some(ids.iter().map(|s| s.to_string()).collect());
        self
    }

    pub(crate) fn data(mut self, data: bool) -> Self {
        self.data = data;
        self
    }

    pub(crate) fn build(mut self, service: &super::Service) -> crate::Result<AttachmentsRequest> {
        let base = service.base();
        let url = match (&self.bug_ids.as_deref(), &self.attachment_ids.as_deref()) {
            (Some([id, ids @ ..]), None) => {
                if !ids.is_empty() {
                    self.params.push(("ids".to_string(), ids.iter().join(",")));
                }
                format!("{base}/rest/bug/{id}/attachment")
            }
            (None, Some([id, ids @ ..])) => {
                if !ids.is_empty() {
                    self.params
                        .push(("attachment_ids".to_string(), ids.iter().join(",")));
                }
                format!("{base}/rest/bug/attachment/{id}")
            }
            _ => {
                return Err(Error::InvalidValue(
                    "invalid attachments IDs state".to_string(),
                ))
            }
        };

        if !self.data {
            self.params
                .push(("exclude_fields".to_string(), "data".to_string()));
        }

        let url = Url::parse_with_params(&url, self.params)
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;

        Ok(AttachmentsRequest {
            bug_ids: self.bug_ids,
            attachment_ids: self.attachment_ids,
            req: service.client().get(url).build()?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct AttachmentsRequest {
    bug_ids: Option<Vec<String>>,
    attachment_ids: Option<Vec<String>>,
    req: reqwest::Request,
}

impl AttachmentsRequest {
    pub(crate) fn builder() -> AttachmentsRequestBuilder {
        AttachmentsRequestBuilder::default()
    }
}

impl Request for AttachmentsRequest {
    type Output = Vec<Attachment>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
        let mut data = service.parse_response(response).await?;
        let mut attachments = vec![];
        match (self.bug_ids, self.attachment_ids) {
            (Some(ids), None) => {
                debug!("attachments request data: {data}");
                let mut data = data["bugs"].take();
                for id in ids {
                    let data = data[&id].take();
                    attachments.extend(serde_json::from_value::<Vec<Attachment>>(data)?);
                }
                Ok(attachments)
            }
            (None, Some(ids)) => {
                debug!("attachments request data: {data}");
                let mut data = data["attachments"].take();
                for id in ids {
                    let data = data[&id].take();
                    let attachment = serde_json::from_value(data)
                        .map_err(|_| Error::InvalidValue(format!("unknown attachment ID: {id}")))?;
                    attachments.push(attachment);
                }
                Ok(attachments)
            }
            _ => panic!("invalid attachments ID state"),
        }
    }
}
