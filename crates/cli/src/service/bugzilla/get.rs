use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use clap::Args;

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
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// open bugs in browser
    #[arg(
        short,
        long,
        long_help = indoc::indoc! {"
            Open bugs in a browser.

            This functionality requires xdg-open with a valid, preferred browser
            set for http(s) URLs.
        "}
    )]
    browser: bool,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(
        required = true,
        help_heading = "Arguments",
        long_help = indoc::indoc! {"
            IDs or aliases of bugs to fetch.

            Taken from standard input when `-`.

            Example:
              - fetch all matching bugs: bite s bugbite -f id | bite g -
        "}
    )]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();

        if self.browser {
            let urls = ids.iter().map(|id| client.service().item_url(id));
            launch_browser(urls)?;
        } else {
            let attachments = !self.options.no_attachments;
            let comments = !self.options.no_comments;
            let history = !self.options.no_history;
            let bugs = client.get(ids, attachments, comments, history).await?;
            render_items(bugs)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
