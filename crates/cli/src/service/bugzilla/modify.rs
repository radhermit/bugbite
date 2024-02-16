use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(required = true, help_heading = "Arguments")]
    // TODO: add stdin support
    ids: Vec<u64>,
}

impl Command {
    pub(super) fn run(self, _client: Client) -> anyhow::Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
