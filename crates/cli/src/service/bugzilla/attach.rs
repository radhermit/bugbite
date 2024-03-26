use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::attach::CreateAttachment;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use tracing::info;

use crate::utils::wrapped_doc;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// comment to add with the attachment
    #[arg(short, long)]
    comment: Option<String>,

    /// specify the MIME type
    #[arg(
        short,
        long,
        conflicts_with = "patch",
        long_help = wrapped_doc!("
            Specify the MIME type of the attachment.

            This option is unnecessary for regular usage since the MIME type is
            automatically detected using `file` with a fallback to internal
            inference of common file types.
        ")
    )]
    mime: Option<String>,

    /// attachment is a patch
    #[arg(short, long, conflicts_with = "mime")]
    patch: bool,

    /// attachment is private
    #[arg(short = 'P', long)]
    private: bool,

    /// short description of the attachment
    #[arg(
        short,
        long,
        long_help = wrapped_doc!("
            A short description of the attachment.

            By default the file name is used when this is not specified.
        ")
    )]
    summary: Option<String>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Arguments")]
struct Arguments {
    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(
        display_order = 0,
        num_args = 1,
        required = true,
        value_delimiter = ',',
        value_name = "ID[,...]",
        long_help = wrapped_doc!("
            IDs or aliases of bugs to create attachments for.

            Taken from standard input when `-`.

            Example:
            - attach to all matching bugs
            > bite s bugbite -f id | bite at - path/to/file

            - attach to multiple bugs
            > bite at 3,4,5 file
        ")
    )]
    ids: Vec<MaybeStdinVec<String>>,

    /// files to attach
    #[clap(
        display_order = 1,
        required = true,
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Paths to attachment files.

            Multiple attachments can be created by specifying multiple paths.

            Example:
            - attach multiple files
            > bite at 3 file1 file2 file3
        ")
    )]
    files: Vec<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    args: Arguments,
}

impl Command {
    pub(super) async fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut attachments = vec![];
        for file in &self.args.files {
            let mut attachment = CreateAttachment::new(file)?;
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

        let ids = &self.args.ids.iter().flatten().collect::<Vec<_>>();
        let attachment_ids = client.attach(ids, attachments).await?;

        let item_ids = ids.iter().map(|x| x.to_string()).join(", ");
        for (file, ids) in self.args.files.iter().zip(attachment_ids.iter()) {
            let ids = ids.iter().map(|x| x.to_string()).join(", ");
            info!("{file}: attached to bug(s): {item_ids} (attachment ID(s) {ids})");
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_examples(&["bugzilla", "attach"]);
    }
}
