use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::service::bugzilla::comment::tag::*;
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

    /// add/remove/set comment tags
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    tags: Option<Vec<SetChange<String>>>,

    /// untag comments
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
        conflicts_with = "tags",
        required_unless_present = "tags",
    )]
    untag: Option<bool>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            attachment: value.attachment,
            created_after: value.created,
            creator: value.creator,
            tags: value.tags,
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
    pub(super) async fn run<W>(self, service: &Bugzilla, _f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.iter().flatten().collect::<Vec<_>>();
        let untag = self.params.untag.unwrap_or_default();
        let mut request = service.comment_tag(&ids);
        request.params = self.params.into();
        if untag {
            request.params.tags = Some(Default::default());
        }
        request.send().await?;
        Ok(ExitCode::SUCCESS)
    }
}
