use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use clap::Args;

use crate::macros::async_block;

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

    /// bug IDs
    #[clap(help_heading = "Arguments")]
    // TODO: add stdin support
    ids: Vec<u64>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let attachments = !self.options.no_attachments;
        let comments = !self.options.no_comments;
        let history = self.options.show_history;
        let bugs = async_block!(client.get(&self.ids, attachments, comments, history))?;
        let mut stdout = stdout().lock();

        for bug in bugs {
            write!(stdout, "{bug}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
