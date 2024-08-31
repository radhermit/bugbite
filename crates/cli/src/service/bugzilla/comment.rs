use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::comment::Parameters;
use bugbite::service::bugzilla::Service;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::RequestSend;
use clap::Args;

use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Args)]
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

#[derive(Args)]
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
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        let ids: Vec<_> = self.ids.iter().flatten().collect();
        let mut request = service.comment(&ids);
        request.params = self.params.into();
        let comments = request.send().await?;
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
                    service.render(comment, &mut stdout, width)?;
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
