use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::io::{stdin, stdout, IsTerminal, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result};
use crossterm::terminal;
use itertools::Itertools;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn confirm<S>(prompt: S, default: bool) -> Result<bool>
where
    S: std::fmt::Display,
{
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
        let value = answer.trim();

        if value.is_empty() {
            return Ok(default);
        } else if value == "Y" || value == "y" {
            return Ok(true);
        } else if value == "N" || value == "n" {
            return Ok(false);
        } else {
            writeln!(stdout, "please answer y or n")?;
        }
    }
}

pub(crate) fn launch_browser<I, S>(urls: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    for url in urls {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("failed launching browser via xdg-open")?;
    }
    Ok(())
}

pub(crate) fn launch_editor<P: AsRef<Path>>(path: P) -> Result<ExitStatus> {
    let path = path.as_ref();
    if let Ok(exe) = env::var("EDITOR") {
        Command::new(&exe)
            .arg(path)
            .status()
            .with_context(|| format!("failed launching editor: {exe}"))
    } else {
        Command::new("xdg-open")
            .arg(path)
            .status()
            .context("failed launching editor via xdg-open")
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

macro_rules! wrapped_doc {
    ($content:expr) => {{
        let options = textwrap::Options::new(80)
            .break_words(false)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);
        textwrap::wrap(indoc::indoc!($content).trim(), &options).join("\n")
    }};
    ($content:expr, $($args:tt)*) => {{
        let options = textwrap::Options::new(80)
            .break_words(false)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);
        textwrap::wrap(indoc::formatdoc!($content, $($args)*).trim(), &options).join("\n")
    }};
}
pub(crate) use wrapped_doc;
