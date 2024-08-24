use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::history::Parameters;
use bugbite::service::bugzilla::Service;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::RequestSend;
use clap::Args;

use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Args)]
#[clap(next_help_heading = "History options")]
struct Params {
    /// event occurred at this time or later
    #[arg(short, long, value_name = "TIME")]
    created: Option<TimeDeltaOrStatic>,

    /// user who made change
    #[arg(short = 'R', long, value_name = "USER")]
    creator: Option<String>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            created_after: value.created,
            creator: value.creator,
        }
    }
}

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        let ids: Vec<_> = self.ids.iter().flatten().collect();
        let events = service
            .history(&ids)
            .params(self.params.into())
            .send()
            .await?;
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
