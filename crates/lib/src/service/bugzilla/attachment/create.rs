use std::fs::{self, File};
use std::process::Command;
use std::{fmt, io, str};

use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde::Serialize;
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::objects::bugzilla::Flag;
use crate::objects::Base64;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

/// Compression variants supported by attachments.
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
    fn cmd(&self) -> &str {
        match self {
            Self::Bz2 => "bzip2",
            Self::Gz => "gzip",
            Self::Lz => "lzip",
            Self::Xz => "xz",
            Self::Zstd => "zstd",
        }
    }

    fn run(&self, path: &Utf8Path, tempdir: &Utf8Path) -> crate::Result<String> {
        let file_name = path
            .file_name()
            .ok_or_else(|| Error::InvalidValue(format!("src missing file name: {path}")))?;
        let src = File::open(path)
            .map_err(|e| Error::InvalidValue(format!("invalid src: {path}: {e}")))?;

        let name = format!("{file_name}.{self}");
        let dest = File::create(tempdir.join(&name))
            .map_err(|e| Error::InvalidValue(format!("failed creating file: {name}: {e}")))?;
        let tool = self.cmd();
        let mut cmd = Command::new(tool);
        cmd.arg("-c").stdin(src).stdout(dest);

        match cmd.status() {
            Ok(status) => {
                if !status.success() {
                    Err(Error::InvalidValue(format!(
                        "failed compressing file: {path}"
                    )))
                } else {
                    Ok(name)
                }
            }
            Err(e) => {
                let msg = if e.kind() == io::ErrorKind::NotFound {
                    format!("{tool} not available")
                } else {
                    e.to_string()
                };

                Err(Error::InvalidValue(format!(
                    "failed compressing file: {path}: {msg}"
                )))
            }
        }
    }
}

// Try to detect data content type use `file` then via `infer, and finally falling back to
// generic text-based vs binary data.
fn get_mime_type<P: AsRef<Utf8Path>>(path: P, data: &[u8]) -> String {
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

/// Create a tarball from a given source directory into a given destination file path.
fn tar<P1, P2>(src: P1, dest_dir: P2) -> crate::Result<String>
where
    P1: AsRef<Utf8Path>,
    P2: AsRef<Utf8Path>,
{
    let src = src.as_ref();
    let dest_dir = dest_dir.as_ref();
    let src = src
        .canonicalize_utf8()
        .map_err(|e| Error::InvalidValue(format!("invalid tarball source: {src}: {e}")))?;
    let src_file_name = src
        .file_name()
        .ok_or_else(|| Error::InvalidValue(format!("invalid tarball source: {src}")))?;
    let src_dir = src
        .parent()
        .ok_or_else(|| Error::InvalidValue(format!("invalid tarball source: {src}")))?;
    let dest_file_name = format!("{src_file_name}.tar");
    let dest = dest_dir.join(&dest_file_name);
    let mut cmd = Command::new("tar");
    cmd.args([
        "-C",
        src_dir.as_str(),
        "-c",
        src_file_name,
        "-f",
        dest.as_str(),
    ]);

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                Err(Error::InvalidValue(format!(
                    "failed creating tarball: {dest}"
                )))
            } else {
                Ok(dest_file_name)
            }
        }
        Err(e) => {
            let msg = if e.kind() == io::ErrorKind::NotFound {
                "tar not available".to_string()
            } else {
                e.to_string()
            };

            Err(Error::InvalidValue(format!(
                "failed creating tarball: {dest}: {msg}"
            )))
        }
    }
}

/// Attachment creation object.
#[derive(Debug)]
pub struct CreateAttachment {
    /// Path to the attachment.
    path: Utf8PathBuf,

    /// Comment related to the attachment.
    pub comment: Option<String>,

    /// Attachment description, by default the submitted file name is used.
    pub description: Option<String>,

    /// Attachment flags.
    pub flags: Option<Vec<Flag>>,

    /// MIME type of the attachment.
    mime_type: Option<String>,

    /// Attachment file name, by default the submitted file name is used.
    pub name: Option<String>,

    /// Attachment is a patch file.
    pub is_patch: Option<bool>,

    /// Mark the attachment private on creation.
    pub is_private: Option<bool>,

    /// Compress the attachment using a given compression type.
    pub compress: Option<Compression>,

    /// Automatically compress the attachment if it exceeds a given size in MB.
    pub auto_compress: Option<f64>,

    /// Automatically truncate plain text attachments if exceeding a number of lines.
    auto_truncate: Option<usize>,
}

impl CreateAttachment {
    /// Create a new attachment using a given path.
    pub fn new<P: AsRef<Utf8Path>>(path: P) -> Self {
        let path = path.as_ref();

        // use default compression for directory targets
        let compress = if path.is_dir() {
            Some(Default::default())
        } else {
            None
        };

        Self {
            path: path.to_path_buf(),
            comment: None,
            description: None,
            flags: None,
            mime_type: None,
            name: None,
            is_patch: None,
            is_private: None,
            compress,
            auto_compress: None,
            auto_truncate: None,
        }
    }

    /// Set the attachment comment.
    pub fn comment<S: fmt::Display>(mut self, value: Option<S>) -> Self {
        self.comment = value.map(|s| s.to_string());
        self
    }

    /// Set the attachment description.
    pub fn flags<I>(mut self, value: Option<I>) -> Self
    where
        I: IntoIterator<Item = Flag>,
    {
        self.flags = value.map(|i| i.into_iter().collect());
        self
    }

    /// Set the attachment description.
    pub fn description<S: fmt::Display>(mut self, value: Option<S>) -> Self {
        self.description = value.map(|s| s.to_string());
        self
    }

    /// Set the attachment MIME type.
    pub fn mime_type<S: fmt::Display>(mut self, value: Option<S>) -> Self {
        self.mime_type = value.map(|s| s.to_string());
        self
    }

    /// Set the attachment name.
    pub fn name<S: fmt::Display>(mut self, value: Option<S>) -> Self {
        self.name = value.map(|s| s.to_string());
        self
    }

    /// Compress the attachment using a given compression type.
    pub fn compress(mut self, value: Option<Compression>) -> Self {
        self.compress = value;
        self
    }

    /// Automatically compress the attachment if it exceeds a given size in MB.
    pub fn auto_compress(mut self, value: Option<f64>) -> Self {
        self.auto_compress = value;
        self
    }

    /// Conditionally truncate a text attachment to the last count of lines.
    ///
    /// If the attachment MIME type does not match text/* this setting is ignored.
    pub fn auto_truncate(mut self, value: Option<usize>) -> Self {
        // inject file size compression trigger if none was specified
        if value.is_some() && self.auto_compress.is_none() {
            self.auto_compress = Some(1.0);
        }
        self.auto_truncate = value;
        self
    }

    /// Attachment is a patch file.
    pub fn is_patch(mut self, value: Option<bool>) -> Self {
        self.is_patch = value;
        self
    }

    /// Mark the attachment private on creation.
    pub fn is_private(mut self, value: Option<bool>) -> Self {
        self.is_private = value;
        self
    }

    /// Build an attachment for request submission.
    fn build(self, ids: &[String], temp_dir_path: &Utf8Path) -> crate::Result<Attachment> {
        let mut path = self.path;
        path = path
            .canonicalize_utf8()
            .map_err(|e| Error::InvalidValue(format!("invalid attachment source: {path}: {e}")))?;
        let mut file_name = path
            .file_name()
            .map(|s| s.to_string())
            .ok_or_else(|| Error::InvalidValue(format!("attachment missing file name: {path}")))?;

        // create directory tarball
        let is_patch = self.is_patch.unwrap_or_default();
        if path.is_dir() {
            if let Some(value) = self.mime_type.as_deref() {
                return Err(Error::InvalidValue(format!(
                    "MIME type invalid for directory targets: {value}"
                )));
            };

            if is_patch {
                return Err(Error::InvalidValue(
                    "patch type invalid for directory targets".to_string(),
                ));
            };

            file_name = tar(&path, temp_dir_path)?;
            path = temp_dir_path.join(&file_name);
        }

        let mut data = fs::read(&path)
            .map_err(|e| Error::InvalidValue(format!("failed reading attachment: {path}: {e}")))?;
        let mut mime_type = get_mime_type(&path, &data);

        // optionally truncate text files
        if let Some(count) = self.auto_truncate {
            if mime_type.starts_with("text/") {
                path = temp_dir_path.join(&file_name);
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

        // determine if a file of a given size will be auto-compressed
        let auto_compress = |bytes: usize| -> bool {
            self.auto_compress
                .map(|x| x * 1e6 < bytes as f64)
                .unwrap_or_default()
        };

        // compress file if forced or triggered by size
        if (self.compress.is_some() && self.auto_compress.is_none()) || auto_compress(data.len()) {
            let compress = self.compress.unwrap_or_default();
            file_name = compress.run(&path, temp_dir_path)?;
            path = temp_dir_path.join(&file_name);
            data = fs::read(&path).map_err(|e| {
                Error::InvalidValue(format!(
                    "failed reading compressed attachment: {file_name}: {e}"
                ))
            })?;
            mime_type = get_mime_type(path, &data);
        }

        Ok(Attachment {
            ids: ids.to_vec(),
            data: Base64(data),
            content_type: self.mime_type.unwrap_or(mime_type),
            file_name: self.name.unwrap_or(file_name.clone()),
            summary: self.description.unwrap_or(file_name),
            comment: self.comment.unwrap_or_default(),
            is_patch,
            is_private: self.is_private.unwrap_or_default(),
            flags: self.flags,
        })
    }
}

/// Attachment object used for request submission.
#[skip_serializing_none]
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
    flags: Option<Vec<Flag>>,
}

#[derive(Debug)]
pub struct Request {
    url: Url,
    attachments: Vec<Attachment>,
}

impl Request {
    pub(crate) fn new<I, S>(
        service: &Service,
        ids: I,
        create_attachments: Vec<CreateAttachment>,
    ) -> crate::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        if create_attachments.is_empty() {
            return Err(Error::InvalidRequest(
                "no attachments specified".to_string(),
            ));
        };

        let ids: Vec<_> = ids.into_iter().map(|s| s.to_string()).collect();
        let id = ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let url = service
            .config
            .base
            .join(&format!("rest/bug/{id}/attachment"))?;

        // create temporary directory used for creating transient attachment files
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::InvalidValue(format!("failed acquiring temporary dir: {e}")))?;
        let temp_dir_path = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| Error::InvalidValue("non-unicode temporary dir path".to_string()))?;

        let attachments: Vec<_> = create_attachments
            .into_iter()
            .map(|x| x.build(&ids, temp_dir_path))
            .try_collect()?;

        Ok(Self { url, attachments })
    }
}

impl RequestSend for Request {
    type Output = Vec<Vec<u64>>;
    type Service = Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .attachments
            .into_iter()
            .map(|x| service.client.post(self.url.clone()).json(&x))
            .map(|r| r.auth(service).map(|r| r.send()))
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
