use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command};

use crossterm::terminal;
use once_cell::sync::Lazy;

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
