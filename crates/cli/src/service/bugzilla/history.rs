use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::time::TimeDelta;
use clap::Args;
use itertools::Itertools;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "History options")]
struct Options {
    /// event occurred at this time or later
    #[arg(short, long, value_name = "TIME")]
    created: Option<TimeDelta>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids: Vec<_> = self.ids.iter().flatten().collect();
        let created = self.options.created.as_ref();
        let events = async_block!(client.history(&ids, created))?;
        let mut stdout = stdout().lock();
        write!(stdout, "{}", events.iter().flatten().join("\n"))?;
        Ok(ExitCode::SUCCESS)
    }
}
