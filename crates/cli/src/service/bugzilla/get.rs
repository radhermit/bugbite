use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use clap::Args;

use crate::macros::async_block;
use crate::service::output::render_items;
use crate::utils::launch_browser;

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
    ids: Vec<MaybeStdinVec<NonZeroU64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();

        if self.options.browser {
            let urls = ids.iter().map(|id| client.item_url(*id));
            launch_browser(urls)?;
        } else {
            let attachments = !self.options.no_attachments;
            let comments = !self.options.no_comments;
            let history = !self.options.no_history;
            let bugs = async_block!(client.get(ids, attachments, comments, history))?;
            render_items(bugs)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
