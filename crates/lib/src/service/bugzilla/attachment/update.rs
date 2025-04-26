use serde::Serialize;
use serde_json::Value;
use serde_with::skip_serializing_none;
use url::Url;

use crate::Error;
use crate::objects::bugzilla::Flag;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<u64>,
    pub params: Parameters,
}

impl Request {
    pub(crate) fn new<I>(service: &Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        Self {
            service: service.clone(),
            ids: ids.into_iter().collect(),
            params: Default::default(),
        }
    }

    /// Encode parameters into the form required for the request.
    fn encode(&self) -> crate::Result<RequestParameters> {
        // verify parameters exist
        if self.params == Parameters::default() {
            return Err(Error::EmptyParams);
        }

        Ok(RequestParameters {
            ids: &self.ids,
            file_name: self.params.name.as_deref(),
            summary: self.params.description.as_deref(),
            comment: self.params.comment.as_deref(),
            content_type: self.params.mime_type.as_deref(),
            is_patch: self.params.patch,
            is_private: self.params.private,
            is_obsolete: self.params.obsolete,
            flags: self.params.flags.as_deref(),
        })
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let url = self
            .service
            .config()
            .base
            .join(&format!("rest/bug/attachment/{id}"))?;

        Ok(url)
    }
}

impl RequestSend for Request {
    type Output = Vec<u64>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.encode()?;
        let request = self
            .service
            .client()
            .put(url)
            .json(&params)
            .auth(&self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(data) = data["attachments"].take() else {
            return Err(Error::InvalidResponse(
                "attachment update request".to_string(),
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

/// Attachment update parameters.
#[derive(Debug, Default, PartialEq, Eq)]
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

/// Internal attachment update request parameters.
#[skip_serializing_none]
#[derive(Serialize)]
struct RequestParameters<'a> {
    ids: &'a [u64],
    file_name: Option<&'a str>,
    summary: Option<&'a str>,
    comment: Option<&'a str>,
    content_type: Option<&'a str>,
    is_patch: Option<bool>,
    is_private: Option<bool>,
    is_obsolete: Option<bool>,
    flags: Option<&'a [Flag]>,
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.attachment_update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }
}
