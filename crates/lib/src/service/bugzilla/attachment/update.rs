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
    fn encode<'a>(&'a self, ids: &'a [String]) -> RequestParameters<'a> {
        RequestParameters {
            ids,
            file_name: self.name.as_deref(),
            summary: self.description.as_deref(),
            comment: self.comment.as_deref(),
            content_type: self.mime_type.as_deref(),
            is_patch: self.patch,
            is_private: self.private,
            is_obsolete: self.obsolete,
            flags: self.flags.as_deref(),
        }
    }
}

/// Internal attachment update request parameters.
#[skip_serializing_none]
#[derive(Serialize)]
struct RequestParameters<'a> {
    ids: &'a [String],
    file_name: Option<&'a str>,
    summary: Option<&'a str>,
    comment: Option<&'a str>,
    content_type: Option<&'a str>,
    is_patch: Option<bool>,
    is_private: Option<bool>,
    is_obsolete: Option<bool>,
    flags: Option<&'a [Flag]>,
}

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    pub ids: Vec<String>,
    pub params: Parameters,
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

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.params.encode(&self.ids);
        let request = self
            .service
            .client
            .put(url)
            .json(&params)
            .auth(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(data) = data["attachments"].take() else {
            return Err(Error::InvalidResponse(
                "invalid service response to attachment update request".to_string(),
            ));
        };

        let mut ids = vec![];
        for mut change in data {
            let id = serde_json::from_value(change["id"].take()).map_err(|e| {
                Error::InvalidResponse(format!("failed deserializing changes: {e}"))
            })?;
            ids.push(id);
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.attachment_update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }
}
