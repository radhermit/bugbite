use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::attachment::update::Parameters;
use clap::Args;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachment options")]
struct Options {
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
            flags: self.options.flags,
            mime_type: self.options.mime,
            name: self.options.name,
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
