use camino::Utf8PathBuf;

use crate::Error;

/// Get the current working directory as an absolute [`Utf8PathBuf`].
pub fn current_dir() -> crate::Result<Utf8PathBuf> {
    Utf8PathBuf::from(".")
        .canonicalize_utf8()
        .map_err(|e| Error::InvalidValue(format!("invalid current working directory: {e}")))
}
