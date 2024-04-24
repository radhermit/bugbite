use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::attachment::update::Parameters;
use clap::Args;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachment options")]
struct Options {
    /// attachment comment
    #[arg(short, long, value_name = "VALUE")]
    comment: Option<String>,

    /// attachment description
    #[arg(short, long, value_name = "VALUE")]
    description: Option<String>,

    /// attachment file name
    #[arg(short, long, value_name = "VALUE")]
    file_name: Option<String>,

    /// attachment MIME type
    #[arg(short, long, value_name = "TYPE", conflicts_with_all = ["patch"])]
    mime: Option<String>,

    /// attachment is obsolete
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
    )]
    obsolete: Option<bool>,

    /// attachment is a patch
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

    /// attachment is private
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

#[derive(Debug, Args)]
#[clap(next_help_heading = "Arguments")]
struct Arguments {
    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// attachment IDs
    #[clap(required = true, value_name = "ID[,...]")]
    ids: Vec<MaybeStdinVec<u64>>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    args: Arguments,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.args.ids.iter().flatten().collect::<Vec<_>>();
        let params = Parameters {
            comment: self.options.comment,
            description: self.options.description,
            file_name: self.options.file_name,
            mime_type: self.options.mime,
            is_obsolete: self.options.obsolete,
            is_patch: self.options.patch,
            is_private: self.options.private,
        };

        let _ = client.attachment_update(ids, params).await?;

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc("bite-bugzilla-attachment-update");
    }
}