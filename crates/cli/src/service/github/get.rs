use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::github::Client;
use clap::Args;

#[derive(Debug, Args)]
pub(super) struct Command {
    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// issue IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();
        let issues = client.get(ids, false, false, false).await?;
        let mut stdout = stdout().lock();

        for issue in issues {
            write!(stdout, "{issue}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
