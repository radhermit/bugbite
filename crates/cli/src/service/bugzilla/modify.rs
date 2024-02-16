use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct Command {
    // TODO: add stdin support
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, _client: Client) -> anyhow::Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
