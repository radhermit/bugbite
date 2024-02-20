use std::process::ExitCode;

pub(crate) mod bugzilla;
pub(crate) mod github;

/// Force authentication and retry if a command has an authentication failure.
fn login_retry<F>(func: F) -> Result<ExitCode, bugbite::Error>
where
    F: Fn() -> Result<ExitCode, bugbite::Error>,
{
    let mut result = func();
    if let Err(bugbite::Error::Auth(_)) = &result {
        // TODO: if unauthenticated, login (if possible) and retry function
        result = func();
    }
    result
}

// TODO: remove this once authentication support is added
#[allow(dead_code)]
/// Force authentication before running a command.
fn login_required<F>(func: F) -> Result<ExitCode, bugbite::Error>
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
