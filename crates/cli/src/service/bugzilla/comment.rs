use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::output::{COLUMNS, Render};
use bugbite::service::bugzilla::Bugzilla;
use bugbite::service::bugzilla::comment::*;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Comment options")]
struct Params {
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
    created: Option<TimeDeltaOrStatic>,

    /// user who commented
    #[arg(short = 'R', long, value_name = "USER")]
    creator: Option<String>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            attachment: value.attachment,
            created_after: value.created,
            creator: value.creator,
        }
    }
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.iter().flatten().collect::<Vec<_>>();
        let mut request = service.comment(&ids);
        request.params = self.params.into();
        let comments = request.send().await?;
        let mut data = ids.iter().zip(comments).peekable();

        // text wrap width
        let width = COLUMNS.min(90);

        while let Some((id, comments)) = data.next() {
            if !comments.is_empty() {
                // output bug ID header
                let bug_id = format!("Bug: {id} ");
                writeln!(f, "{bug_id:=<width$}")?;

                let mut comments_iter = comments.iter().peekable();
                while let Some(comment) = comments_iter.next() {
                    // render comment
                    comment.render(f, width)?;
                    // add new line between comments
                    if comments_iter.peek().is_some() {
                        writeln!(f)?;
                    }
                }

                // add new line between bugs
                if data.peek().is_some() {
                    writeln!(f)?;
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
