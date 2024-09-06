use std::process::Command;
use std::str;

use camino::{Utf8Path, Utf8PathBuf};

use crate::Error;

/// Get the user config directory for bugbite.
pub fn config_dir() -> crate::Result<Utf8PathBuf> {
    let config_dir = dirs_next::config_dir()
        .ok_or_else(|| Error::InvalidValue("failed getting config directory".to_string()))?;
    let config_dir = Utf8PathBuf::from_path_buf(config_dir)
        .map_err(|e| Error::InvalidValue(format!("invalid bugbite config directory: {e:?}")))?;

    Ok(config_dir.join("bugbite"))
}

/// Get the current working directory as an absolute [`Utf8PathBuf`].
pub fn current_dir() -> crate::Result<Utf8PathBuf> {
    Utf8PathBuf::from(".")
        .canonicalize_utf8()
        .map_err(|e| Error::InvalidValue(format!("invalid current working directory: {e}")))
}

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

/// Return true if a given file descriptor is a terminal/tty, otherwise false.
///
/// Allows overriding the return value for testing purposes.
#[macro_export]
macro_rules! is_terminal {
    ($fd:expr) => {
        std::io::IsTerminal::is_terminal($fd)
            || (cfg!(feature = "test") && std::env::var("BUGBITE_IS_TERMINAL").is_ok())
    };
}
pub use is_terminal;
