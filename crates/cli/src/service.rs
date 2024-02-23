use std::process::ExitCode;

// output and rendering support
pub(crate) mod output;

// service modules
pub(crate) mod bugzilla;
pub(crate) mod github;
pub(crate) mod redmine;

/// Force authentication and retry if a command has an authentication failure.
fn auth_retry<F>(mut func: F) -> Result<ExitCode, bugbite::Error>
where
    F: FnMut() -> Result<ExitCode, bugbite::Error>,
{
    let mut result = func();
    if let Err(bugbite::Error::Auth) = &result {
        // TODO: if unauthenticated, authenticate (if possible) and retry function
        result = func();
    }
    result
}

/// Force authentication before running a command.
fn auth_required<F>(func: F) -> Result<ExitCode, bugbite::Error>
where
    F: Fn() -> Result<ExitCode, bugbite::Error>,
{
    // TODO: force authentication
    func()
}

/// Render an object for output to the terminal.
pub(crate) trait Render {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()>;
}
