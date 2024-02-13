use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command};

use crossterm::terminal;
use once_cell::sync::Lazy;
use textwrap::{wrap, Options};

/// Format a list of possible option values for --help and man page output.
pub(crate) fn possible_values(values: &[&str]) -> String {
    let data = values.join(", ");
    let options = Options::new(60)
        .initial_indent("  ")
        .subsequent_indent("  ")
        .break_words(false);
    wrap(&data, &options).join("\n")
}

pub(crate) fn launch_browser<I, S>(urls: I) -> io::Result<Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("xdg-open").args(urls).spawn()
}

pub(crate) static COLUMNS: Lazy<usize> = Lazy::new(|| {
    let (cols, _rows) = terminal::size().unwrap_or_default();
    cols.into()
});
