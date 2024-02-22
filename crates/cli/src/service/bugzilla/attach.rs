use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::attach::CreateAttachment;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use tracing::info;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// short description of the attachment
    #[arg(short, long)]
    summary: Option<String>,

    /// comment to add with the attachment
    #[arg(short, long)]
    comment: Option<String>,

    /// specify the MIME type
    #[arg(short, long, conflicts_with = "patch")]
    mime: Option<String>,

    /// attachment is a patch
    #[arg(short, long, conflicts_with = "mime")]
    patch: bool,

    /// attachment is private
    #[arg(short = 'P', long)]
    private: bool,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// bug IDs
    #[clap(
        required = true,
        value_name = "ID[,ID,...]",
        help_heading = "Arguments"
    )]
    ids: Csv<u64>,

    /// attachment paths
    #[clap(
        required = true,
        value_hint = ValueHint::FilePath,
        help_heading = "Arguments"
    )]
    files: Vec<Utf8PathBuf>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids: Vec<_> = self.ids.iter().copied().collect();
        let mut attachments = vec![];
        for file in &self.files {
            let mut attachment = CreateAttachment::new(&ids, file)?;
            if let Some(value) = self.options.summary.as_ref() {
                attachment.summary = value.clone()
            }
            if let Some(value) = self.options.comment.as_ref() {
                attachment.comment = value.clone()
            }
            if let Some(value) = self.options.mime.as_ref() {
                attachment.content_type = value.clone()
            }
            attachment.is_patch = self.options.patch;
            attachment.is_private = self.options.private;
            attachments.push(attachment);
        }

        let attachment_ids = async_block!(client.attach(attachments))?;

        let item_ids = ids.iter().map(|x| x.to_string()).join(", ");
        for (file, ids) in self.files.iter().zip(attachment_ids.iter()) {
            let ids = ids.iter().map(|x| x.to_string()).join(", ");
            info!("{file}: attached to bug(s): {item_ids} (attachment ID(s) {ids})");
        }

        Ok(ExitCode::SUCCESS)
    }
}
