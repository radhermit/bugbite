use std::process::ExitCode;

use bugbite::client::Client;
use bugbite::service;

use crate::options::Options;

mod get;
mod modify;
mod search;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(super) fn run(
        self,
        _options: Options,
        service: service::Config,
    ) -> anyhow::Result<ExitCode> {
        let client = Client::builder().build(service)?;
        self.cmd.run(client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Modify issues
    #[command(alias = "m")]
    Modify(modify::Command),
    /// Search issues
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(client),
            Self::Modify(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
