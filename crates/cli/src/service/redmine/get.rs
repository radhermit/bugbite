use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use clap::builder::BoolishValueParser;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::{launch_browser, COLUMNS};

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// enable/disable comments
    #[arg(
        short = 'C',
        long,
        value_name = "BOOL",
        default_value = "true",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        hide_possible_values = true,
    )]
    comments: Option<bool>,

    /// launch in browser
    #[arg(short, long, default_value_t = false)]
    browser: bool,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// issue IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids: Vec<_> = self.ids.iter().flatten().collect();

        if self.options.browser {
            let urls = ids.iter().map(|id| client.item_url(id));
            launch_browser(urls)?;
        } else {
            let comments = self.options.comments.unwrap_or_default();
            let issues = async_block!(client.get(&ids, false, comments))?;
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
