use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use bugbite::time::TimeDelta;
use clap::Args;
use itertools::Itertools;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Comment options")]
struct Options {
    /// comment created at this time or later
    #[arg(short, long, value_name = "TIME")]
    created: Option<TimeDelta>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// bug IDs
    #[clap(required = true, help_heading = "Arguments")]
    // TODO: add stdin support
    ids: Vec<u64>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let events = async_block!(client.comments(&self.ids, self.options.created))?;
        let mut stdout = stdout().lock();
        write!(stdout, "{}", events.iter().join("\n"))?;
        Ok(ExitCode::SUCCESS)
    }
}