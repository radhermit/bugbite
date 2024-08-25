use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::io::{stdin, stdout, BufRead, Write};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result};
use bugbite::utils::is_terminal;
use crossterm::terminal;
use itertools::Itertools;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn confirm<S>(prompt: S, default: bool) -> Result<bool>
where
    S: std::fmt::Display,
{
    let mut stdout = stdout().lock();
    let mut stdin = stdin().lock();
    let vals = if default { "Y/n" } else { "y/N" };
    loop {
        write!(stdout, "{prompt} ({vals}): ")?;
        stdout.flush()?;
        let mut answer = String::new();
        stdin.read_line(&mut answer)?;
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
    let browser = env::var("BROWSER").unwrap_or_default();
    let args = shlex::split(&browser).unwrap_or_default();

    for url in urls {
        if !args.is_empty() {
            let cmd = &args[0];
            Command::new(cmd)
                .args(&args[1..])
                .arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("failed launching browser via {cmd}"))?;
        } else {
            Command::new("xdg-open")
                .arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("failed launching browser via xdg-open")?;
        }
    }

    Ok(())
}

pub(crate) fn launch_editor<S: AsRef<OsStr>>(path: S) -> Result<ExitStatus> {
    let editor = env::var("EDITOR").unwrap_or_default();
    let args = shlex::split(&editor).unwrap_or_default();
    if !args.is_empty() {
        let cmd = &args[0];
        Command::new(cmd)
            .args(&args[1..])
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .with_context(|| format!("failed launching editor via {cmd}"))
    } else {
        Command::new("xdg-open")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
    if data.len() > width && is_terminal!(&stdout()) {
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
