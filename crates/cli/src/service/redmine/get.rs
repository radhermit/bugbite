use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use bugbite::traits::Request;
use clap::Args;

use crate::service::output::render_items;
use crate::utils::launch_browser;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// disable comments
    #[arg(short = 'C', long)]
    no_comments: bool,

    /// open in browser
    #[arg(short, long)]
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
    pub(super) async fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();

        if self.options.browser {
            let urls = ids.iter().map(|id| client.service().item_url(id));
            launch_browser(urls)?;
        } else {
            let comments = !self.options.no_comments;
            let request = client.service().get(ids, false, comments)?;
            let issues = request.send(client.service()).await?;
            render_items(issues)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc("bite-redmine-get");
    }
}
