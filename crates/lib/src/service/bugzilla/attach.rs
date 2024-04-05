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
    Lz,
    #[default]
    Xz,
    Zstd,
}

impl Compression {
    fn run(&self, path: &Utf8Path, tempdir: &Utf8Path) -> crate::Result<String> {
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
            Self::Lz => {
                let mut cmd = Command::new("lzip");
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
    pub auto_truncate: Option<usize>,
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
            auto_truncate: None,
        })
    }

    /// Compress an attachment using a specified compression type.
    pub fn compress(&mut self, compress: Compression) {
        self.compress = Some(compress);
    }

    /// Conditionally compress an attachment if larger than a given size in MB.
    pub fn auto_compress(&mut self, size: f64) {
        self.auto_compress = Some(size);
    }

    /// Conditionally truncate a text attachment to the last count of lines.
    ///
    /// If the attachment MIME type does not match text/* this setting is ignored.
    pub fn auto_truncate(&mut self, count: usize) {
        // inject file size compression trigger if none was specified
        if self.auto_compress.is_none() {
            self.auto_compress = Some(1.0);
        }
        self.auto_truncate = Some(count);
    }

    fn build<S>(self, ids: &[S]) -> crate::Result<Attachment>
    where
        S: std::fmt::Display,
    {
        let mut path = self.path;
        let mut file_name = path
            .file_name()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::InvalidValue(format!("attachment missing file name: {path}")))?;
        let mut data = fs::read(&path)
            .map_err(|e| Error::InvalidValue(format!("failed reading attachment: {path}: {e}")))?;
        let mut mime_type = get_mime_type(&path, &data);

        // determine file size
        let auto_compress = |size: usize| -> bool {
            self.auto_compress
                .map(|x| x * 1e6 < size as f64)
                .unwrap_or_default()
        };

        // compress and/or truncate the file if requested
        if self.compress.is_some() || auto_compress(data.len()) || self.auto_truncate.is_some() {
            let compress = self.compress.unwrap_or_default();
            let dir = tempfile::tempdir()
                .map_err(|e| Error::InvalidValue(format!("failed acquiring temporary dir: {e}")))?;
            let dir_path = Utf8Path::from_path(dir.path())
                .ok_or_else(|| Error::InvalidValue("non-unicode temporary dir path".to_string()))?;

            // optionally truncate text files
            if let Some(count) = self.auto_truncate {
                if mime_type.starts_with("text/") {
                    path = dir_path.join(&file_name);
                    let s = String::from_utf8(data).map_err(|e| {
                        Error::InvalidValue(format!("invalid attachment file: {path}: {e}"))
                    })?;
                    let content: Vec<_> = s.lines().rev().take(count).collect();
                    data = content.into_iter().rev().join("\n").into_bytes();
                    fs::write(&path, &data).map_err(|e| {
                        Error::InvalidValue(format!("failed writing truncated file: {e}"))
                    })?;
                }
            }

            if self.compress.is_some() || auto_compress(data.len()) {
                file_name = compress.run(&path, dir_path)?;
                let path = dir_path.join(&file_name);
                data = fs::read(&path).map_err(|e| {
                    Error::InvalidValue(format!(
                        "failed reading compressed attachment: {file_name}: {e}"
                    ))
                })?;
                mime_type = get_mime_type(path, &data);
            }
        }

        Ok(Attachment {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            data: Base64(data),
            content_type: mime_type,
            file_name: file_name.clone(),
            summary: self.summary.unwrap_or(file_name),
            comment: self.comment.unwrap_or_default(),
            is_patch: self.is_patch,
            is_private: self.is_private,
        })
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
pub(crate) struct AttachRequest {
    url: Url,
    attachments: Vec<Attachment>,
}

impl AttachRequest {
    pub(crate) fn new<S>(
        service: &super::Service,
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

        let mut attachments = vec![];
        for attachment in create_attachments {
            attachments.push(attachment.build(ids)?);
        }

        Ok(Self { url, attachments })
    }
}

impl Request for AttachRequest {
    type Output = Vec<Vec<u64>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .attachments
            .into_iter()
            .map(|x| service.client().post(self.url.clone()).json(&x))
            .map(|r| r.inject_auth(service, true).map(|r| r.send()))
            .try_collect()?;

        let mut attachment_ids = vec![];
        for future in futures {
            let response = future.await?;
            let mut data = service.parse_response(response).await?;
            let data = data["ids"].take();
            let ids = serde_json::from_value(data)
                .map_err(|e| Error::InvalidValue(format!("failed deserializing ids: {e}")))?;
            attachment_ids.push(ids);
        }

        Ok(attachment_ids)
    }
}
