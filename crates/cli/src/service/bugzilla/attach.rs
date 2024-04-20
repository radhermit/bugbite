use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::attach::{Compression, CreateAttachment};
use camino::Utf8PathBuf;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, ValueHint};
use itertools::Itertools;
use strum::VariantNames;
use tracing::info;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachment options")]
struct Options {
    /// attachment comment
    #[arg(short, long)]
    comment: Option<String>,

    /// compress attachment
    #[arg(
        short = 'C',
        long,
        conflicts_with_all = ["mime", "patch"],
        num_args = 0..=1,
        default_missing_value = "xz",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(Compression::VARIANTS)
            .map(|s| s.parse::<Compression>().unwrap()),
    )]
    compress: Option<Compression>,

    /// auto-compress attachment
    #[arg(
        long,
        value_name = "SIZE",
        num_args = 0..=1,
        default_missing_value = "1.0",
        conflicts_with_all = ["mime", "patch"],
    )]
    auto_compress: Option<f64>,

    /// auto-truncate text attachment
    #[arg(
        long,
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "1000",
        conflicts_with_all = ["mime", "patch"],
    )]
    auto_truncate: Option<usize>,

    /// support directory targets
    #[arg(short, long, conflicts_with = "mime")]
    dir: bool,

    /// specify the MIME type
    #[arg(
        short,
        long,
        conflicts_with_all = ["compress", "auto_compress", "auto_truncate", "dir", "patch"],
    )]
    mime: Option<String>,

    /// attachment is a patch
    #[arg(
        short,
        long,
        conflicts_with_all = ["compress", "auto_compress", "auto_truncate", "mime"],
    )]
    patch: bool,

    /// attachment is private
    #[arg(short = 'P', long)]
    private: bool,

    /// short description of the attachment
    #[arg(short, long)]
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
        value_name = "ID[,...]"
    )]
    ids: Vec<MaybeStdinVec<String>>,

    /// files to attach
    #[clap(
        display_order = 1,
        required = true,
        value_hint = ValueHint::FilePath,
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
            let mut attachment = CreateAttachment::new(file);
            attachment.summary = self.options.summary.clone();
            attachment.comment = self.options.comment.clone();
            attachment.content_type = self.options.mime.clone();
            attachment.dir = self.options.dir;
            attachment.is_patch = self.options.patch;
            attachment.is_private = self.options.private;
            attachment.compress = self.options.compress;
            attachment.auto_compress = self.options.auto_compress;
            if let Some(value) = self.options.auto_truncate {
                attachment.auto_truncate(value);
            }

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
        subcmd_parse_doc(&["bugzilla", "attach"]);
    }
}
