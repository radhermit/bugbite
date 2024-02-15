use std::env;

use camino::Utf8PathBuf;

use crate::Error;

/// Get the current working directory as [`Utf8PathBuf`].
pub fn current_dir() -> crate::Result<Utf8PathBuf> {
    let dir = env::current_dir()
        .map_err(|e| Error::InvalidValue(format!("can't get current dir: {e}")))?;
    Utf8PathBuf::try_from(dir)
        .map_err(|e| Error::InvalidValue(format!("invalid unicode path: {e}")))
}
