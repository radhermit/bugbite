use std::fmt;

use chrono::prelude::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use url::Url;

use crate::Error;
use crate::serde::non_empty_str;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, ParseResponse, RequestSend};

pub use crate::service::bugzilla::objects::Flag;

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,

    /// Attachment IDs.
    pub ids: Vec<u64>,

    /// Request parameters.
    pub params: Parameters,
}

impl Request {
    /// Create a new request.
    pub(crate) fn new<I>(service: Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        Self {
            service,
            ids: ids.into_iter().collect(),
            params: Default::default(),
        }
    }

    /// Encode parameters into the form required for the request.
    fn encode(&self) -> crate::Result<RequestParameters<'_>> {
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

    /// Generate the URL for the request.
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

    /// Update the attachment comment.
    pub fn comment<S>(&mut self, value: S) -> &mut Self
    where
        S: fmt::Display,
    {
        self.params.comment = Some(value.to_string());
        self
    }

    /// Update the attachment description.
    pub fn description<S>(&mut self, value: S) -> &mut Self
    where
        S: fmt::Display,
    {
        self.params.description = Some(value.to_string());
        self
    }

    /// Update the attachment MIME type.
    pub fn mime_type<S>(&mut self, value: S) -> &mut Self
    where
        S: fmt::Display,
    {
        self.params.mime_type = Some(value.to_string());
        self
    }

    /// Update the attachment file name.
    pub fn name<S>(&mut self, value: S) -> &mut Self
    where
        S: fmt::Display,
    {
        self.params.name = Some(value.to_string());
        self
    }

    /// Update the attachment obsolete status.
    pub fn obsolete(&mut self, value: bool) -> &mut Self {
        self.params.obsolete = Some(value);
        self
    }

    /// Update the attachment patch status.
    pub fn patch(&mut self, value: bool) -> &mut Self {
        self.params.patch = Some(value);
        self
    }

    /// Update the attachment private status.
    pub fn private(&mut self, value: bool) -> &mut Self {
        self.params.private = Some(value);
        self
    }
}

#[serde_as]
#[derive(Deserialize, Debug)]
struct Change {
    #[serde(deserialize_with = "non_empty_str")]
    #[serde(rename = "added")]
    _added: Option<String>,

    #[serde(deserialize_with = "non_empty_str")]
    #[serde(rename = "removed")]
    _removed: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AttachmentChanges {
    /// Attachment ID
    id: u64,

    /// Attachment update time
    #[serde(rename = "last_change_time")]
    _updated: DateTime<Utc>,

    /// Attachment changes made
    #[serde(rename = "changes")]
    _changes: IndexMap<String, Change>,
}

/// Bugzilla REST API response to an attachment update request.
///
/// https://bugzilla.readthedocs.io/en/latest/api/core/v1/attachment.html#update-attachment
#[derive(Deserialize, Debug)]
struct Response {
    attachments: Vec<AttachmentChanges>,
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
        let data: Response = self.service.parse_response(response).await?;

        let mut ids = vec![];
        for change in data.attachments {
            ids.push(change.id);
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
    use std::assert_matches;

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.attachment_update(ids).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no IDs specified");
    }
}
