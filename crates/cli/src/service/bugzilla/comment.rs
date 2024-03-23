use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::comment::CommentParams;
use bugbite::time::TimeDelta;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Comment options")]
struct Options {
    /// comment includes attachment
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
    )]
    attachment: Option<bool>,

    /// comment created at this time or later
    #[arg(short, long, value_name = "TIME")]
    created: Option<TimeDelta>,

    /// user who commented
    #[arg(short = 'R', long, value_name = "USER")]
    creator: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();

        let mut params = CommentParams::new();

        if let Some(value) = self.options.attachment {
            params.attachment(value);
        }

        if let Some(value) = self.options.created {
            params.created_after(value);
        }

        if let Some(value) = self.options.creator {
            params.creator(value);
        }

        let comments = async_block!(client.comment(ids, Some(params)))?;
        let mut comments = comments.iter().flatten().peekable();
        let mut stdout = stdout().lock();

        // text wrap width
        let width = if *COLUMNS <= 90 { *COLUMNS } else { 90 };
        while let Some(comment) = comments.next() {
            comment.render(&mut stdout, width)?;
            if comments.peek().is_some() {
                writeln!(stdout)?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
