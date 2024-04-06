use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::comment::CommentParams;
use bugbite::time::TimeDelta;
use clap::Args;

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
        hide_possible_values = true,
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
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
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

        let comments = client.comment(ids, Some(params)).await?;
        let mut data = ids.iter().zip(comments).peekable();
        let mut stdout = stdout().lock();

        // text wrap width
        let width = if *COLUMNS <= 90 { *COLUMNS } else { 90 };

        while let Some((id, comments)) = data.next() {
            if !comments.is_empty() {
                // output bug ID header
                let bug_id = format!("Bug: {id} ");
                writeln!(stdout, "{bug_id}{}", "=".repeat(width - bug_id.len()))?;

                let mut comments_iter = comments.iter().peekable();
                while let Some(comment) = comments_iter.next() {
                    // render comment
                    comment.render(&mut stdout, width)?;
                    // add new line between comments
                    if comments_iter.peek().is_some() {
                        writeln!(stdout)?;
                    }
                }

                // add new line between bugs
                if data.peek().is_some() {
                    writeln!(stdout)?;
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_examples(&["bugzilla", "comment"]);
    }
}
