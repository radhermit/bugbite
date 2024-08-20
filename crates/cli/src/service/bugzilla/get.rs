use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use clap::Args;

use crate::service::output::render_items;
use crate::utils::launch_browser;

#[derive(Args)]
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

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// open in browser
    #[arg(short, long)]
    browser: bool,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run(&self, service: &Service) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();

        if self.browser {
            let urls = ids.iter().map(|id| service.item_url(id));
            launch_browser(urls)?;
        } else {
            let bugs = service
                .get(ids)
                .attachments(!self.options.no_attachments)
                .comments(!self.options.no_comments)
                .history(!self.options.no_history)
                .send()
                .await?;
            render_items(bugs)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc("bite-bugzilla-get");
    }
}
