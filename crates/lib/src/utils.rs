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

/// Merge two Option wrapped values together, the second value overrides the first if it exists.
macro_rules! or {
    ($orig:expr, $new:expr) => {
        if $new.is_some() {
            $orig = $new;
        }
    };
}
pub(crate) use or;

/// Prefix a string with a given value if missing.
macro_rules! prefix {
    ($prefix:expr, $value:expr) => {{
        let prefix = $prefix;
        let value = $value.to_string();
        if !value.starts_with($prefix) {
            format!("{prefix}{value}")
        } else {
            value
        }
    }};
}
pub(crate) use prefix;

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
