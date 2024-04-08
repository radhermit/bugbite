use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::history::HistoryParams;
use bugbite::time::TimeDeltaOrStatic;
use clap::Args;

use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Debug, Args)]
#[clap(next_help_heading = "History options")]
struct Options {
    /// event occurred at this time or later
    #[arg(short, long, value_name = "TIME")]
    created: Option<TimeDeltaOrStatic>,

    /// user who made change
    #[arg(short = 'R', long, value_name = "USER")]
    creator: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();

        let mut params = HistoryParams::new();

        if let Some(value) = self.options.created {
            params.created_after(value);
        }

        if let Some(value) = self.options.creator {
            params.creator(value);
        }

        let events = client.history(ids, Some(params)).await?;
        let mut data = ids.iter().zip(events).peekable();
        let mut stdout = stdout().lock();

        // text wrap width
        let width = if *COLUMNS <= 90 { *COLUMNS } else { 90 };

        while let Some((id, events)) = data.next() {
            if !events.is_empty() {
                // output bug ID header
                let bug_id = format!("Bug: {id} ");
                writeln!(stdout, "{bug_id}{}", "=".repeat(width - bug_id.len()))?;

                let mut events = events.iter().peekable();
                while let Some(event) = events.next() {
                    // render event
                    event.render(&mut stdout, width)?;
                    // add new line between events
                    if events.peek().is_some() {
                        writeln!(stdout)?;
                    }
                }

                // add new line between bugs
                if data.peek().is_some() {
                    writeln!(stdout)?;
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_examples(&["bugzilla", "history"]);
    }
}
