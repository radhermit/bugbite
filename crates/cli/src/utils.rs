use std::borrow::Cow;
use std::ffi::OsStr;
use std::io::{self, stdout, IsTerminal};
use std::process::{Child, Command};

use crossterm::terminal;
use itertools::Itertools;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

#[allow(dead_code)]
pub(crate) fn launch_browser<I, S>(urls: I) -> io::Result<Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("xdg-open").args(urls).spawn()
}

pub(crate) static COLUMNS: Lazy<usize> = Lazy::new(|| {
    let (cols, _rows) = terminal::size().unwrap_or((80, 24));
    // use a static width when testing is enabled
    if cfg!(feature = "test") {
        80
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
