use std::path::Path;
use std::process::Command;
use std::str;

use camino::Utf8PathBuf;

use crate::Error;

/// Get the current working directory as an absolute [`Utf8PathBuf`].
pub fn current_dir() -> crate::Result<Utf8PathBuf> {
    Utf8PathBuf::from(".")
        .canonicalize_utf8()
        .map_err(|e| Error::InvalidValue(format!("invalid current working directory: {e}")))
}

/// Try to get the MIME type of a file path using the `file` utility.
pub(crate) fn get_mime_type<P: AsRef<Path>>(path: P) -> crate::Result<String> {
    let output = Command::new("file")
        .args(["-b", "--mime-type"])
        .arg(path.as_ref())
        .output()?;

    match str::from_utf8(&output.stdout).map(|s| s.trim()) {
        Ok(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(Error::InvalidValue(
            "file command returned invalid value".to_string(),
        )),
    }
}
