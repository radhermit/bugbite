use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::attach::CreateAttachment;
use camino::Utf8PathBuf;
use clap::Args;
use itertools::Itertools;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// comment to add with the attachment
    #[arg(short, long)]
    comment: Option<String>,

    /// short description of the attachment
    #[arg(short, long)]
    summary: Option<String>,

    /// specify the MIME type
    #[arg(short, long, conflicts_with = "patch")]
    mime: Option<String>,

    /// attachment is a patch
    #[arg(short, long, conflicts_with = "mime")]
    patch: Option<bool>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(required = true, help_heading = "Arguments")]
    path: Utf8PathBuf,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids: Vec<_> = self.ids.iter().flatten().copied().collect();
        let mut stdout = stdout().lock();
        let attachment = CreateAttachment::new(ids, &self.path)?;

        let ids = async_block!(client.attach(attachment))?;
        let ids = ids.iter().map(|x| x.to_string()).join(", ");
        writeln!(stdout, "{} attached to: {ids}", self.path)?;

        Ok(ExitCode::SUCCESS)
    }
}
