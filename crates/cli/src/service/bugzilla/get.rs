use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use clap::builder::BoolishValueParser;
use clap::Args;

use crate::macros::async_block;
use crate::utils::COLUMNS;

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

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: MaybeStdinVec<u64>,
    #[clap(hide = true, value_name = "IDS")]
    ids2: Vec<u64>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids = &[&self.ids[..], &self.ids2].concat();
        let attachments = self.options.attachments.unwrap_or_default();
        let comments = self.options.comments.unwrap_or_default();
        let history = self.options.history.unwrap_or_default();
        let bugs = async_block!(client.get(ids, attachments, comments, history,))?;
        let mut bugs = bugs.into_iter().peekable();
        let mut stdout = stdout().lock();

        while let Some(bug) = bugs.next() {
            write!(stdout, "{bug}")?;
            if bugs.peek().is_some() {
                writeln!(stdout, "{}", "=".repeat(*COLUMNS))?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
