use std::process::ExitCode;

use bugbite::client::github::Client;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<u64>,
}

impl Command {
    pub(super) fn run(self, _client: Client) -> anyhow::Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
