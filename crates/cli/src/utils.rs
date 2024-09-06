use std::borrow::Cow;
use std::collections::VecDeque;
use std::env;
use std::ffi::OsStr;
use std::io::{stderr, stdin, BufRead, IsTerminal, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::AtomicBool;

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
    let mut stderr = stderr().lock();
    let mut stdin = stdin().lock();
    let vals = if default { "Y/n" } else { "y/N" };
    loop {
        write!(stderr, "{prompt} ({vals}): ")?;
        stderr.flush()?;
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
            writeln!(stderr, "please answer y or n")?;
        }
    }
}

pub(crate) fn launch_browser<I, S>(urls: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let browser = env::var("BROWSER").unwrap_or_default();
    let mut args = shlex::split(&browser)
        .unwrap_or_default()
        .into_iter()
        .collect::<VecDeque<_>>();
    let cmd = args.pop_front();
    let cmd = cmd.as_deref().unwrap_or("xdg-open");

    for url in urls {
        Command::new(cmd)
            .args(&args)
            .arg(url.as_ref())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed launching browser via {cmd}"))?;
    }

    Ok(())
}

pub(crate) fn launch_editor<S: AsRef<OsStr>>(path: S) -> Result<ExitStatus> {
    let editor = env::var("EDITOR").unwrap_or_default();
    let mut args = shlex::split(&editor)
        .unwrap_or_default()
        .into_iter()
        .collect::<VecDeque<_>>();
    let cmd = args.pop_front();
    let cmd = cmd.as_deref().unwrap_or("xdg-open");

    Command::new(cmd)
        .args(&args)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed launching editor via {cmd}"))
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
pub(crate) fn truncate<'a, W>(f: &W, data: &'a str, width: usize) -> Cow<'a, str>
where
    W: IsTerminal,
{
    if data.len() > width && is_terminal!(f) {
        let mut iter = UnicodeSegmentation::graphemes(data, true).take(*COLUMNS);
        Cow::Owned(iter.join(""))
    } else {
        Cow::Borrowed(data)
    }
}

pub(crate) static VERBOSE: AtomicBool = AtomicBool::new(false);

macro_rules! verbose {
    ($dst:expr, $($arg:tt)+) => {
        if $crate::utils::VERBOSE.load(std::sync::atomic::Ordering::Acquire) {
            writeln!($dst, $($arg)+)
        } else {
            Ok(())
        }
    };
    ($enable:expr) => {
        $crate::utils::VERBOSE.store($enable, std::sync::atomic::Ordering::SeqCst);
    };
    () => {
        $crate::utils::VERBOSE.load(std::sync::atomic::Ordering::Acquire)
    };
}
pub(crate) use verbose;

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
