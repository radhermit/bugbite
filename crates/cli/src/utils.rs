use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::io::{self, stdin, stdout, IsTerminal, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

use crossterm::terminal;
use itertools::Itertools;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn confirm(prompt: &str, default: bool) -> io::Result<bool> {
    let mut answer = String::new();
    let mut stdout = stdout().lock();
    loop {
        if default {
            write!(stdout, "{prompt} (Y/n): ")?;
        } else {
            write!(stdout, "{prompt} (y/N): ")?;
        }

        stdout.flush()?;
        stdin().read_line(&mut answer)?;

        if answer.trim().is_empty() {
            return Ok(default);
        } else if &answer == "Y" || &answer == "y" {
            return Ok(true);
        } else if &answer == "N" || &answer == "n" {
            return Ok(false);
        } else {
            writeln!(stdout, "please answer y or n")?;
        }
    }
}

pub(crate) fn launch_browser<I, S>(urls: I) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    for url in urls {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

pub(crate) fn launch_editor<P: AsRef<Path>>(path: P) -> io::Result<ExitStatus> {
    let path = path.as_ref();
    if let Ok(exe) = env::var("EDITOR") {
        Command::new(exe).arg(path).status()
    } else {
        Command::new("xdg-open").arg(path).status()
    }
}

pub(crate) static COLUMNS: Lazy<usize> = Lazy::new(|| {
    let (cols, _rows) = terminal::size().unwrap_or((90, 24));
    // use a static width when testing is enabled
    if cfg!(feature = "test") {
        90
    } else {
        cols.into()
    }
});

/// Truncate a string to the requested width of graphemes.
pub(crate) fn truncate(data: &str, width: usize) -> Cow<'_, str> {
    if data.len() > width && stdout().is_terminal() {
        let mut iter = UnicodeSegmentation::graphemes(data, true).take(*COLUMNS);
        Cow::Owned(iter.join(""))
    } else {
        Cow::Borrowed(data)
    }
}
