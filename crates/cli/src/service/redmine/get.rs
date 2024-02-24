use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::{launch_browser, COLUMNS};

#[derive(Debug, Args)]
pub(super) struct Command {
    /// launch in browser
    #[arg(short, long, default_value_t = false)]
    browser: bool,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// issue IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids: Vec<_> = self.ids.iter().flatten().collect();

        if self.browser {
            let urls = ids.iter().map(|id| client.item_url(id));
            launch_browser(urls)?;
        } else {
            let issues = async_block!(client.get(&ids, false))?;
            let mut issues = issues.into_iter().peekable();
            let mut stdout = stdout().lock();

            // text wrap width
            let width = if stdout.is_terminal() && *COLUMNS <= 90 && *COLUMNS >= 50 {
                *COLUMNS
            } else {
                90
            };

            while let Some(issue) = issues.next() {
                issue.render(&mut stdout, width)?;
                if issues.peek().is_some() {
                    writeln!(stdout, "{}", "=".repeat(width))?;
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
