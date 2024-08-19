use std::process::Command;
use std::str;

use camino::{Utf8Path, Utf8PathBuf};

use crate::Error;

/// Get the current working directory as an absolute [`Utf8PathBuf`].
pub fn current_dir() -> crate::Result<Utf8PathBuf> {
    Utf8PathBuf::from(".")
        .canonicalize_utf8()
        .map_err(|e| Error::InvalidValue(format!("invalid current working directory: {e}")))
}

/// Merge two Option wrapped values together, the second value is used if the first is None.
#[macro_export]
macro_rules! or {
    ($orig:expr, $new:expr) => {
        if $orig.is_none() {
            $orig = $new;
        }
    };
}
pub(crate) use or;

/// Try to get the MIME type of a file path using the `file` utility.
///
/// Note that `file` can misidentify plain text file types as various text/* subtypes depending
/// on formatting within the file.
pub(crate) fn get_mime_type<P: AsRef<Utf8Path>>(path: P) -> crate::Result<String> {
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
