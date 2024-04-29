use serde::Serialize;
use serde_json::Value;
use serde_with::skip_serializing_none;
use url::Url;

use crate::objects::bugzilla::Flag;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

/// Attachment update parameters.
#[derive(Debug, Default)]
pub struct Parameters {
    /// Comment related to the attachment.
    pub comment: Option<String>,

    /// Attachment description.
    pub description: Option<String>,

    /// Attachment flags.
    pub flags: Option<Vec<Flag>>,

    /// MIME type of the attachment.
    pub mime_type: Option<String>,

    /// Attachment file name.
    pub name: Option<String>,

    /// Attachment is obsolete.
    pub is_obsolete: Option<bool>,

    /// Attachment is a patch file.
    pub is_patch: Option<bool>,

    /// Mark the attachment private on creation.
    pub is_private: Option<bool>,
}

impl Parameters {
    /// Encode parameters into the form required for the request.
    fn encode(self, ids: Vec<String>) -> RequestParameters {
        RequestParameters {
            ids,
            file_name: self.name,
            summary: self.description,
            comment: self.comment,
            content_type: self.mime_type,
            is_patch: self.is_patch,
            is_private: self.is_private,
            is_obsolete: self.is_obsolete,
            flags: self.flags,
        }
    }
}

/// Internal attachment update request parameters.
#[skip_serializing_none]
#[derive(Serialize, Debug)]
struct RequestParameters {
    ids: Vec<String>,
    file_name: Option<String>,
    summary: Option<String>,
    comment: Option<String>,
    content_type: Option<String>,
    is_patch: Option<bool>,
    is_private: Option<bool>,
    is_obsolete: Option<bool>,
    flags: Option<Vec<Flag>>,
}

#[derive(Debug)]
pub struct AttachmentUpdateRequest {
    url: Url,
    ids: Vec<String>,
    params: Parameters,
}

impl AttachmentUpdateRequest {
    pub(crate) fn new<S>(service: &Service, ids: &[S], params: Parameters) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let [id, ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        Ok(Self {
            url: service
                .config
                .base
                .join(&format!("rest/bug/attachment/{id}"))?,
            ids: ids.iter().map(|x| x.to_string()).collect(),
            params,
        })
    }
}

impl RequestSend for AttachmentUpdateRequest {
    type Output = Vec<u64>;
    type Service = Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let params = self.params.encode(self.ids);
        let request = service.client.put(self.url).json(&params).auth(service)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let Value::Array(data) = data["attachments"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to attachment update request".to_string(),
            ));
        };

        let mut ids = vec![];
        for mut change in data {
            let id = serde_json::from_value(change["id"].take())
                .map_err(|e| Error::InvalidValue(format!("failed deserializing changes: {e}")))?;
            ids.push(id);
        }

        Ok(ids)
    }
}
