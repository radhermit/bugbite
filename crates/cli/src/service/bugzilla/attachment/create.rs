use std::process::ExitCode;

use bugbite::args::CsvOrStdin;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::attachment::create::{Attachment, Compression};
use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
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
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        conflicts_with_all = ["compress", "auto_compress", "auto_truncate", "mime"],
    )]
    patch: Option<bool>,

    /// attachment is private
    #[arg(
        short = 'P',
        long,
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
    )]
    private: Option<bool>,
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
    /// bug IDs or aliases
    #[clap(display_order = 0, required = true, value_name = "ID[,...]")]
    ids: CsvOrStdin<String>,

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
        let attachment_ids = service
            .attachment_create(&self.args.ids)
            .attachments(self.args.files.iter().map(|file| {
                Attachment::new(file)
                    .comment(self.options.comment.as_deref())
                    .description(self.options.description.as_deref())
                    .flags(self.options.flags.clone())
                    .mime_type(self.options.mime.as_deref())
                    .name(self.options.name.as_deref())
                    .compress(self.compression.compress)
                    .auto_compress(self.compression.auto_compress)
                    .auto_truncate(self.compression.auto_truncate)
                    .is_patch(self.options.patch)
                    .is_private(self.options.private)
            }))
            .send()
            .await?;

        let item_ids = self.args.ids.iter().join(", ");
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
