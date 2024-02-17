use std::process::ExitCode;

use bugbite::client::{Client, ClientBuilder};
use bugbite::service::ServiceKind;
use tracing::info;

mod attachments;
mod comments;
mod get;
mod history;
mod search;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) fn run(
        self,
        kind: ServiceKind,
        base: String,
        client: ClientBuilder,
    ) -> anyhow::Result<ExitCode> {
        let service = kind.create(&base)?;
        info!("{service}");
        self.cmd.run(client.build(service)?)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get attachments
    #[command(visible_alias = "a")]
    Attachments(attachments::Command),
    /// Get comments
    Comments(comments::Command),
    /// Get bugs
    #[command(visible_alias = "g")]
    Get(get::Command),
    /// Get bug history
    History(history::Command),
    /// Search bugs
    #[command(visible_alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let client = client.into_bugzilla().unwrap();
        match self {
            Self::Attachments(cmd) => cmd.run(client),
            Self::Comments(cmd) => cmd.run(client),
            Self::Get(cmd) => cmd.run(client),
            Self::History(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
