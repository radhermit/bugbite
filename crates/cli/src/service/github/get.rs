use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::output::render_items;
use bugbite::service::github::Github;
use bugbite::traits::RequestSend;
use clap::Args;

use crate::utils::launch_browser;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// open in browser
    #[arg(short, long)]
    browser: bool,
}

#[derive(Args, Debug)]
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
    pub(super) async fn run<W>(self, service: &Github, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.into_iter().flatten();

        if self.options.browser {
            let urls = ids.map(|id| service.item_url(id));
            launch_browser(urls)?;
        } else {
            let issues = service.get(ids).send().await?;
            render_items(f, &issues)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
