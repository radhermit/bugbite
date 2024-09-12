use std::collections::VecDeque;
use std::env;
use std::ffi::OsStr;
use std::io::{stderr, stdin, BufRead, Write};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result};

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
