use std::fs;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::objects::Base64;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct CreateAttachment {
    ids: Vec<u64>,
    data: Base64,
    file_name: String,
    content_type: String,
    pub summary: String,
    pub comment: String,
    pub is_patch: bool,
    pub is_private: bool,
}

impl CreateAttachment {
    pub fn new<P>(ids: Vec<u64>, path: P) -> crate::Result<Self>
    where
        P: AsRef<Utf8Path>,
    {
        let path = path.as_ref();
        let data = fs::read(path)
            .map_err(|e| Error::InvalidValue(format!("failed reading attachment: {path}: {e}")))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::InvalidValue(format!("attachment missing file name: {path}")))?;
        let kind = infer::get(&data)
            .ok_or_else(|| Error::InvalidValue(format!("unknown attachment format: {path}")))?;

        Ok(Self {
            ids,
            data: Base64(data),
            file_name: file_name.to_string(),
            content_type: kind.mime_type().to_string(),
            summary: Default::default(),
            comment: Default::default(),
            is_patch: Default::default(),
            is_private: Default::default(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct AttachRequest {
    url: Url,
    attachment: CreateAttachment,
}

impl AttachRequest {
    pub(crate) fn new(
        service: &super::Service,
        attachment: CreateAttachment,
    ) -> crate::Result<Self> {
        let [id, _remaining_ids @ ..] = &attachment.ids[..] else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let url = service.base().join(&format!("/rest/bug/{id}/attachment"))?;

        Ok(Self { url, attachment })
    }
}

impl Request for AttachRequest {
    type Output = Vec<u64>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service
            .client()
            .post(self.url)
            .json(&self.attachment)
            .send()
            .await?;
        let mut data = service.parse_response(response).await?;
        let data = data["ids"].take();
        Ok(serde_json::from_value(data)?)
    }
}
