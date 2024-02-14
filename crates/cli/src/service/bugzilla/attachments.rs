use std::process::ExitCode;

use bugbite::client::Client;
use camino::Utf8PathBuf;
use clap::Args;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// output attachment data
    #[arg(short = 'V', long)]
    view: bool,

    /// search by bug ID
    #[arg(short, long)]
    bug_id: bool,

    /// save attachments to a specified directory
    #[arg(short, long, value_name = "PATH")]
    dir: Utf8PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// attachment IDs
    #[clap(help_heading = "Arguments")]
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, _client: Client) -> anyhow::Result<ExitCode> {
        Ok(ExitCode::SUCCESS)
    }
}
