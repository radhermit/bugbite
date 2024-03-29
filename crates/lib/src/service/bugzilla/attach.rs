use std::{fs, str};

use camino::Utf8Path;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::objects::Base64;
use crate::traits::{InjectAuth, Request, WebService};
use crate::utils::get_mime_type;
use crate::Error;

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct CreateAttachment {
    ids: Vec<String>,
    data: Base64,
    file_name: String,
    pub content_type: String,
    pub summary: String,
    pub comment: String,
    pub is_patch: bool,
    pub is_private: bool,
}

impl CreateAttachment {
    pub fn new<P>(path: P) -> crate::Result<Self>
    where
        P: AsRef<Utf8Path>,
    {
        let path = path.as_ref();
        let data = fs::read(path)
            .map_err(|e| Error::InvalidValue(format!("failed reading attachment: {path}: {e}")))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::InvalidValue(format!("attachment missing file name: {path}")))?;

        // Try to detect data content type use `file` then via `infer, and finally falling back to
        // generic text-based vs binary data.
        let mime_type = if let Ok(value) = get_mime_type(path) {
            value
        } else if let Some(kind) = infer::get(&data) {
            kind.mime_type().to_string()
        } else if str::from_utf8(&data).is_ok() {
            "text/plain".to_string()
        } else {
            "application/octet-stream".to_string()
        };

        Ok(Self {
            ids: Default::default(),
            data: Base64(data),
            file_name: file_name.to_string(),
            content_type: mime_type,
            summary: file_name.to_string(),
            comment: Default::default(),
            is_patch: Default::default(),
            is_private: Default::default(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct AttachRequest<'a> {
    url: Url,
    attachments: Vec<CreateAttachment>,
    service: &'a super::Service,
}

impl<'a> AttachRequest<'a> {
    pub(crate) fn new<S>(
        service: &'a super::Service,
        ids: &[S],
        mut attachments: Vec<CreateAttachment>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        if attachments.is_empty() {
            return Err(Error::InvalidRequest(
                "no attachments specified".to_string(),
            ));
        };

        let [id, ..] = &ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let url = service.base().join(&format!("rest/bug/{id}/attachment"))?;

        for attachment in &mut attachments {
            attachment.ids = ids.iter().map(|x| x.to_string()).collect();
        }

        Ok(Self {
            url,
            attachments,
            service,
        })
    }
}

impl Request for AttachRequest<'_> {
    type Output = Vec<Vec<u64>>;

    async fn send(self) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .attachments
            .into_iter()
            .map(|x| self.service.client().post(self.url.clone()).json(&x))
            .map(|r| r.inject_auth(self.service, true).map(|r| r.send()))
            .try_collect()?;

        let mut attachment_ids = vec![];
        for future in futures {
            let response = future.await?;
            let mut data = self.service.parse_response(response).await?;
            let data = data["ids"].take();
            let ids = serde_json::from_value(data)
                .map_err(|e| Error::InvalidValue(format!("failed deserializing ids: {e}")))?;
            attachment_ids.push(ids);
        }

        Ok(attachment_ids)
    }
}
