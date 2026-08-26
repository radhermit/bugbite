use std::collections::VecDeque;
use std::env;
use std::ffi::OsStr;
use std::io::{self, BufRead, Write};
use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result};

fn confirm_inner<S, R, W>(
    prompt: S,
    default: bool,
    mut reader: R,
    mut writer: W,
) -> io::Result<bool>
where
    S: std::fmt::Display,
    R: BufRead,
    W: Write,
{
    let vals = if default { "Y/n" } else { "y/N" };
    loop {
        write!(writer, "{prompt} ({vals}): ")?;
        writer.flush()?;
        let mut answer = String::new();
        reader.read_line(&mut answer)?;
        let value = answer.trim();

        match value {
            "" => return Ok(default),
            "Y" | "y" => return Ok(true),
            "N" | "n" => return Ok(false),
            _ => writeln!(writer, "please answer y or n")?,
        }
    }
}

/// Requests interactive user confirmation.
pub(crate) fn confirm<S>(prompt: S, default: bool) -> io::Result<bool>
where
    S: std::fmt::Display,
{
    confirm_inner(prompt, default, io::stdin().lock(), io::stderr().lock())
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_confirm() {
        // default true
        let input = Cursor::new("\n"); // user hits Enter immediately
        let mut output = Vec::new();
        let result = confirm_inner("Proceed?", true, input, &mut output);
        assert!(result.unwrap());
        let output_str = str::from_utf8(&output).unwrap();
        assert!(output_str.contains("Proceed? (Y/n): "));

        // explicit yes
        for value in ["y", "Y"] {
            let input = Cursor::new(format!("{value}\n"));
            let mut output = Vec::new();
            let result = confirm_inner("Proceed?", true, input, &mut output);
            assert!(result.unwrap());
        }

        // explicit no
        for value in ["n", "N"] {
            let input = Cursor::new(format!("{value}\n"));
            let mut output = Vec::new();
            let result = confirm_inner("Proceed?", true, input, &mut output);
            assert!(!result.unwrap());
        }

        // confirm retry on invalid, first input is invalid ("maybe"), second input is valid ("y")
        let input = Cursor::new("maybe\ny\n");
        let mut output = Vec::new();
        let result = confirm_inner("Proceed?", false, input, &mut output);
        assert!(result.unwrap());
        let output_str = str::from_utf8(&output).unwrap();
        assert!(output_str.contains("please answer y or n"));
    }
}
