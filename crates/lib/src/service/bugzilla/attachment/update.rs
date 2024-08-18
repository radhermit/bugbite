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
    pub obsolete: Option<bool>,

    /// Attachment is a patch file.
    pub patch: Option<bool>,

    /// Mark the attachment private on creation.
    pub private: Option<bool>,
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
            is_patch: self.patch,
            is_private: self.private,
            is_obsolete: self.obsolete,
            flags: self.flags,
        }
    }
}

/// Internal attachment update request parameters.
#[skip_serializing_none]
#[derive(Serialize)]
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
pub struct Request<'a> {
    service: &'a Service,
    ids: Vec<String>,
    params: Parameters,
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
            params: Default::default(),
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let url = self
            .service
            .config
            .base
            .join(&format!("rest/bug/attachment/{id}"))?;

        Ok(url)
    }

    pub fn params(mut self, params: Parameters) -> Self {
        self.params = params;
        self
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<u64>;

    async fn send(self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.params.encode(self.ids);
        let request = self
            .service
            .client
            .put(url)
            .json(&params)
            .auth(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
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
