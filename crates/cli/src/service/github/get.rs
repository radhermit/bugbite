use std::process::ExitCode;

use bugbite::client::Client;
use clap::Args;
use tokio::runtime::Handle;
use tokio::task;

#[derive(Debug, Args)]
pub(super) struct Command {
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let issues = task::block_in_place(move || {
            Handle::current().block_on(async { client.get(&self.ids, false, false).await })
        })?;

        for issue in issues {
            print!("{issue}");
        }

        Ok(ExitCode::SUCCESS)
    }
}
