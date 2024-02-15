use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command};

use crossterm::terminal;
use once_cell::sync::Lazy;

/// Reset the SIGPIPE to the default behavior.
pub(crate) fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

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
    cols.into()
});
