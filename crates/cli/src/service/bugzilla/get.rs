use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::{launch_browser, COLUMNS};

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// disable attachments
    #[arg(short = 'A', long)]
    no_attachments: bool,

    /// disable comments
    #[arg(short = 'C', long)]
    no_comments: bool,

    /// disable history
    #[arg(short = 'H', long)]
    no_history: bool,

    /// open bugs in browser
    #[arg(short, long)]
    browser: bool,
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

        if self.options.browser {
            let urls = ids.iter().map(|id| client.item_url(id));
            launch_browser(urls)?;
        } else {
            let attachments = !self.options.no_attachments;
            let comments = !self.options.no_comments;
            let history = !self.options.no_history;
            let bugs = async_block!(client.get(&ids, attachments, comments, history))?;
            let mut bugs = bugs.into_iter().peekable();
            let mut stdout = stdout().lock();

            // text wrap width
            let width = if stdout.is_terminal() && *COLUMNS <= 90 && *COLUMNS >= 50 {
                *COLUMNS
            } else {
                90
            };

            while let Some(bug) = bugs.next() {
                bug.render(&mut stdout, width)?;
                if bugs.peek().is_some() {
                    writeln!(stdout, "{}", "=".repeat(width))?;
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
