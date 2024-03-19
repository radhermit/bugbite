use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use clap::Args;

use crate::macros::async_block;
use crate::service::output::render_items;
use crate::utils::launch_browser;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// disable comments
    #[arg(short = 'C', long)]
    no_comments: bool,

    /// open issues in browser
    #[arg(
        short,
        long,
        long_help = indoc::indoc! {"
            Open issues in a browser.

            This functionality requires xdg-open with a valid, preferred browser
            set for http(s) URLs.
        "}
    )]
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
    ids: Vec<MaybeStdinVec<NonZeroU64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();

        if self.options.browser {
            let urls = ids.iter().map(|id| client.item_url(*id));
            launch_browser(urls)?;
        } else {
            let comments = !self.options.no_comments;
            let issues = async_block!(client.get(ids, false, comments))?;
            render_items(issues)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
