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

/// Force authentication before running a command.
fn login_required<F>(func: F) -> Result<ExitCode, bugbite::Error>
where
    F: Fn() -> Result<ExitCode, bugbite::Error>,
{
    // TODO: force authentication
    func()
}
