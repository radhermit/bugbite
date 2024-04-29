use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::attachment::create::{Compression, CreateAttachment};
use bugbite::service::bugzilla::Service;
use bugbite::traits::Request;
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

    /// attachment description
    #[arg(short, long)]
    description: Option<String>,

    /// attachment flags
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    flags: Option<Vec<Flag>>,

    /// attachment MIME type
    #[arg(
        short,
        long,
        value_name = "TYPE",
        conflicts_with_all = ["compress", "auto_compress", "auto_truncate", "patch"],
    )]
    mime: Option<String>,

    /// attachment file name
    #[arg(short, long, value_name = "VALUE")]
    name: Option<String>,

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
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Compression options")]
struct CompressionOptions {
    /// compress attachment
    #[arg(
        short = 'C',
        long,
        num_args = 0..=1,
        default_missing_value = "xz",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(Compression::VARIANTS)
            .map(|s| s.parse::<Compression>().unwrap()),
        conflicts_with_all = ["mime", "patch"],
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
    compression: CompressionOptions,

    #[clap(flatten)]
    args: Arguments,
}

impl Command {
    pub(super) async fn run(&self, service: &Service) -> anyhow::Result<ExitCode> {
        let mut attachments = vec![];
        for file in &self.args.files {
            let mut attachment = CreateAttachment::new(file);
            attachment.comment = self.options.comment.clone();
            attachment.description = self.options.description.clone();
            attachment.flags = self.options.flags.clone();
            if let Some(value) = self.options.mime.as_deref() {
                attachment.mime_type(value)?;
            }
            attachment.name = self.options.name.clone();
            attachment.is_patch = self.options.patch;
            attachment.is_private = self.options.private;
            attachment.compress = self.compression.compress;
            attachment.auto_compress = self.compression.auto_compress;
            if let Some(value) = self.compression.auto_truncate {
                attachment.auto_truncate(value);
            }

            attachments.push(attachment);
        }

        let ids = &self.args.ids.iter().flatten().collect::<Vec<_>>();
        let request = service.attachment_create(ids, attachments)?;
        let attachment_ids = request.send(service).await?;

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
        subcmd_parse_doc("bite-bugzilla-attachment-create");
    }
}
