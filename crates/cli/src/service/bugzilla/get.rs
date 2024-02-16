use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use clap::builder::BoolishValueParser;
use clap::Args;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Get options")]
struct Options {
    /// enable/disable attachments
    #[arg(
        short = 'A',
        long,
        value_name = "BOOL",
        default_value = "true",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        hide_possible_values = true,
    )]
    attachments: Option<bool>,

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

    /// enable/disable history
    #[arg(
        short = 'H',
        long,
        value_name = "BOOL",
        default_value = "false",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        hide_possible_values = true,
    )]
    history: Option<bool>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// bug IDs
    #[clap(required = true, help_heading = "Arguments")]
    // TODO: add stdin support
    ids: Vec<u64>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let attachments = self.options.attachments.unwrap_or_default();
        let comments = self.options.comments.unwrap_or_default();
        let history = self.options.history.unwrap_or_default();
        let bugs = async_block!(client.get(&self.ids, attachments, comments, history,))?;
        let mut stdout = stdout().lock();

        for bug in bugs {
            write!(stdout, "{bug}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
