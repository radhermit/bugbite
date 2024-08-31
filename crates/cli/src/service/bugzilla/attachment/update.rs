use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::attachment::update::Parameters;
use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args)]
#[clap(next_help_heading = "Attachment options")]
struct Params {
    /// update comment
    #[arg(short, long, value_name = "VALUE")]
    comment: Option<String>,

    /// update description
    #[arg(short, long, value_name = "VALUE")]
    description: Option<String>,

    /// update flags
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    flags: Option<Vec<Flag>>,

    /// update MIME type
    #[arg(short, long, value_name = "TYPE", conflicts_with_all = ["patch"])]
    mime: Option<String>,

    /// update file name
    #[arg(short, long, value_name = "VALUE")]
    name: Option<String>,

    /// update obsolete status
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
    )]
    obsolete: Option<bool>,

    /// update patch status
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
        conflicts_with_all = ["mime"],
    )]
    patch: Option<bool>,

    /// update private status
    #[arg(
        short = 'P',
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
    )]
    private: Option<bool>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            comment: value.comment,
            description: value.description,
            flags: value.flags,
            mime_type: value.mime,
            name: value.name,
            obsolete: value.obsolete,
            patch: value.patch,
            private: value.private,
        }
    }
}

#[derive(Args)]
#[clap(next_help_heading = "Arguments")]
struct Arguments {
    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// attachment IDs
    #[clap(required = true, value_name = "ID[,...]")]
    ids: Vec<MaybeStdinVec<u64>>,
}

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,

    #[clap(flatten)]
    args: Arguments,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Service, _f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = &self.args.ids.iter().flatten().collect::<Vec<_>>();
        let mut request = service.attachment_update(ids);
        request.params = self.params.into();
        request.send().await?;
        Ok(ExitCode::SUCCESS)
    }
}
