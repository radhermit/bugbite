use std::process::ExitCode;

use bugbite::client::Client;
use bugbite::service::ServiceKind;
use tracing::info;

use crate::options::Options;

mod attachments;
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
        kind: ServiceKind,
        base: String,
    ) -> anyhow::Result<ExitCode> {
        let service = kind.create(&base)?;
        info!("{service}");
        let client = Client::builder().build(service)?;
        self.cmd.run(client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get attachments from bugs
    #[command(alias = "a")]
    Attachments(attachments::Command),
    /// Get bugs
    #[command(alias = "g")]
    Get(get::Command),
    /// Modify bugs
    #[command(alias = "m")]
    Modify(modify::Command),
    /// Search bugs
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Attachments(cmd) => cmd.run(client),
            Self::Get(cmd) => cmd.run(client),
            Self::Modify(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
