use std::process::ExitCode;

use bugbite::client::github::Client;
use clap::Args;

use crate::macros::async_block;

#[derive(Debug, Args)]
pub(super) struct Command {
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let issues = async_block!(client.get(&self.ids, false, false))?;

        for issue in issues {
            print!("{issue}");
        }

        Ok(ExitCode::SUCCESS)
    }
}
