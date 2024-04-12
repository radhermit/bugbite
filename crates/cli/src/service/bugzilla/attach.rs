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

use crate::utils::wrapped_doc;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// comment to add with the attachment
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
        long_help = wrapped_doc!("
            Compress attachments.

            The value must be the compression variant to use or can be skipped
            to use the default compression: xz.

            Examples modifying bug 10:
            - compress attachment using the default compression type
            > bite at 10 path/to/file --compress

            - compress attachment using zstd
            > bite at 10 path/to/file --compress zstd

            Possible values: {}",
            Compression::VARIANTS.join(", ")
        )
    )]
    compress: Option<Compression>,

    /// auto-compress attachment
    #[arg(
        long,
        value_name = "SIZE",
        num_args = 0..=1,
        default_missing_value = "1",
        conflicts_with_all = ["mime", "patch"],
        long_help = wrapped_doc!("
            Auto-compress attachments larger than a given size.

            The value must be the file size limit in MB above which attachments
            will be compressed, defaulting to 1MB when not given.

            Examples modifying bug 10:
            - auto-compress attachment using the default compression type and size limit
            > bite at 10 path/to/file --auto-compress

            - auto-compress attachment using zstd with 5MB size limit
            > bite at 10 path/to/file --auto-compress 5 --compress zstd
        ")
    )]
    auto_compress: Option<f64>,

    /// auto-truncate text attachment
    #[arg(
        long,
        value_name = "LINES",
        num_args = 0..=1,
        default_missing_value = "1000",
        conflicts_with_all = ["mime", "patch"],
        long_help = wrapped_doc!("
            Auto-truncate text attachments to a given number of lines.

            The value must be the number of lines to which the file will be
            truncated starting from the end, defaulting to 1000 lines when not
            given.

            This option works in coordination with --auto-compress using the
            file size limit to trigger when a text file is truncated. If the
            option is not specified the default value will be used for it.

            Examples modifying bug 10:
            - auto-truncate to 1000 lines
            > bite at 10 path/to/file.txt --auto-truncate

            - auto-truncate to 5000 lines and compress attachment using zstd
            > bite at 10 path/to/file --auto-truncate 5000 --compress zstd
        ")
    )]
    auto_truncate: Option<usize>,

    /// support directory targets
    #[arg(
        short,
        long,
        conflicts_with = "mime",
        long_help = wrapped_doc!("
            Support directory targets for attachments.

            Targeting a directory will attach a compressed tarball of the given
            path. Without this option enabled, directory attachments will cause
            errors.

            Examples modifying bug 10:
            - attach compressed tarball
            > bite at 10 path/to/dir --dir

            - attach tarball compressed with zstd
            > bite at 10 path/to/dir --dir --compress zstd
        ")
    )]
    dir: bool,

    /// specify the MIME type
    #[arg(
        short,
        long,
        conflicts_with_all = ["compress", "auto_compress", "auto_truncate", "dir", "patch"],
        long_help = wrapped_doc!("
            Specify the MIME type of the attachment.

            This option is unnecessary for regular usage since the MIME type is
            automatically detected using `file` with a fallback to internal
            inference of common file types.
        ")
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
        subcmd_parse_examples(&["bugzilla", "attach"]);
    }
}
