use std::io::{stdout, Write};
use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::time::TimeDelta;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::COLUMNS;

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
    ids: Vec<MaybeStdinVec<NonZeroU64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();
        let created = self.options.created.as_ref();
        let events = async_block!(client.history(ids, created))?;
        let mut events = events.iter().flatten().peekable();
        let mut stdout = stdout().lock();

        // text wrap width
        let width = if *COLUMNS <= 90 { *COLUMNS } else { 90 };
        while let Some(event) = events.next() {
            event.render(&mut stdout, width)?;
            if events.peek().is_some() {
                writeln!(stdout)?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
