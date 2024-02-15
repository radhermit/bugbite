use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use clap::Args;
use tokio::runtime::Handle;
use tokio::task;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// disable attachments
    #[arg(short = 'A', long)]
    no_attachments: bool,

    /// disable comments
    #[arg(short = 'C', long)]
    no_comments: bool,

    /// show bug history
    #[arg(short = 'H', long)]
    show_history: bool,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// bug IDs or aliases
    #[clap(help_heading = "Arguments")]
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let comments = !self.options.no_comments;
        let attachments = !self.options.no_attachments;

        let bugs = task::block_in_place(move || {
            Handle::current().block_on(async { client.get(&self.ids, comments, attachments).await })
        })?;

        for bug in bugs {
            print!("{bug}");
        }

        Ok(ExitCode::SUCCESS)
    }
}
