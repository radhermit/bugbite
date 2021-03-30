use std::process::ExitCode;

use bugbite::client::Client;
use clap::Args;

#[derive(Debug, Args)]
pub struct Command {
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, _client: &Client) -> anyhow::Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
