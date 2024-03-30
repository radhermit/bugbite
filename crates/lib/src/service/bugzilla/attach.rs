use std::fs::{self, File};
use std::path::Path;
use std::process::Command;
use std::str;

use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde::Serialize;
use strum::{Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::objects::Base64;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(
    Display, EnumIter, EnumString, VariantNames, Default, Eq, PartialEq, Debug, Clone, Copy,
)]
#[strum(serialize_all = "lowercase")]
pub enum Compression {
    Bz2,
    Gz,
    #[default]
    Xz,
    Zstd,
}

impl Compression {
    fn run(&self, path: &Utf8Path, tempdir: &Path) -> crate::Result<String> {
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::InvalidValue(format!("src missing file name: {path}")))?;
        let src = File::open(path)
            .map_err(|e| Error::InvalidValue(format!("invalid src: {path}: {e}")))?;

        let name = format!("{file_name}.{self}");
        let dest = File::create(tempdir.join(&name)).unwrap();

        let mut cmd = match self {
            Self::Bz2 => {
                let mut cmd = Command::new("bzip2");
                cmd.arg("-c").stdin(src).stdout(dest);
                cmd
            }
            Self::Gz => {
                let mut cmd = Command::new("gzip");
                cmd.arg("-c").stdin(src).stdout(dest);
                cmd
            }
            Self::Xz => {
                let mut cmd = Command::new("xz");
                cmd.arg("-c").stdin(src).stdout(dest);
                cmd
            }
            Self::Zstd => {
                let mut cmd = Command::new("zstd");
                cmd.arg("-c").stdin(src).stdout(dest);
                cmd
            }
        };

        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    return Err(Error::InvalidValue(format!(
                        "failed compressing file: {path}"
                    )));
                }
            }
            Err(e) => {
                return Err(Error::InvalidValue(format!(
                    "failed compressing file: {path}: {e}"
                )));
            }
        }

        Ok(name)
    }
}

#[derive(Debug)]
pub struct CreateAttachment {
    path: Utf8PathBuf,
    pub summary: Option<String>,
    pub content_type: Option<String>,
    pub comment: Option<String>,
    pub is_patch: bool,
    pub is_private: bool,
    pub compress: Option<Compression>,
    pub auto_compress: Option<f64>,
}

// Try to detect data content type use `file` then via `infer, and finally falling back to
// generic text-based vs binary data.
fn get_mime_type<P: AsRef<Path>>(path: P, data: &[u8]) -> String {
    if let Ok(value) = crate::utils::get_mime_type(path) {
        value
    } else if let Some(kind) = infer::get(data) {
        kind.mime_type().to_string()
    } else if str::from_utf8(data).is_ok() {
        "text/plain".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

impl CreateAttachment {
    pub fn new<P>(path: P) -> crate::Result<Self>
    where
        P: AsRef<Utf8Path>,
    {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            summary: None,
            content_type: None,
            comment: None,
            is_patch: false,
            is_private: false,
            compress: None,
            auto_compress: None,
        })
    }

    fn build(self, ids: Vec<String>) -> crate::Result<Attachment> {
        let path = &self.path;
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::InvalidValue(format!("attachment missing file name: {path}")))?;
        let data = fs::read(path)
            .map_err(|e| Error::InvalidValue(format!("failed reading attachment: {path}: {e}")))?;
        let mime_type = get_mime_type(path, &data);

        let mut attachment = Attachment {
            ids,
            data: Base64(data),
            file_name: file_name.to_string(),
            content_type: mime_type,
            summary: self.summary.unwrap_or_else(|| file_name.to_string()),
            comment: self.comment.unwrap_or_default(),
            is_patch: self.is_patch,
            is_private: self.is_private,
        };

        // determine file size
        let f = File::open(path)
            .map_err(|e| Error::InvalidValue(format!("invalid attachment file: {path}: {e}")))?;
        let metadata = f.metadata().unwrap();
        let file_size = metadata.len();
        let auto_compress = self
            .auto_compress
            .map(|x| x * 1e6 < file_size as f64)
            .unwrap_or_default();

        // compress the file if requested
        if self.compress.is_some() || auto_compress {
            let compress = self.compress.unwrap_or_default();
            let dir = tempfile::tempdir()
                .map_err(|e| Error::InvalidValue(format!("failed acquiring temporary dir: {e}")))?;
            let name = compress.run(path, dir.path())?;
            let path = dir.path().join(&name);
            let data = fs::read(&path).map_err(|e| {
                Error::InvalidValue(format!("failed reading compressed attachment: {name}: {e}"))
            })?;
            let mime_type = get_mime_type(path, &data);

            // override attachment fields
            attachment.data = Base64(data);
            attachment.file_name = name.clone();
            attachment.content_type = mime_type;
            if attachment.summary == file_name {
                attachment.summary = name.clone();
            }
        }

        Ok(attachment)
    }
}

#[derive(Serialize, Debug)]
struct Attachment {
    ids: Vec<String>,
    data: Base64,
    file_name: String,
    content_type: String,
    summary: String,
    comment: String,
    is_patch: bool,
    is_private: bool,
}

#[derive(Debug)]
pub(crate) struct AttachRequest<'a> {
    url: Url,
    attachments: Vec<Attachment>,
    service: &'a super::Service,
}

impl<'a> AttachRequest<'a> {
    pub(crate) fn new<S>(
        service: &'a super::Service,
        ids: &[S],
        create_attachments: Vec<CreateAttachment>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        if create_attachments.is_empty() {
            return Err(Error::InvalidRequest(
                "no attachments specified".to_string(),
            ));
        };

        let [id, ..] = &ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let url = service.base().join(&format!("rest/bug/{id}/attachment"))?;

        let ids: Vec<_> = ids.iter().map(|s| s.to_string()).collect();
        let mut attachments = vec![];
        for attachment in create_attachments {
            attachments.push(attachment.build(ids.clone())?);
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
