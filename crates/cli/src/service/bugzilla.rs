use std::process::ExitCode;

use bugbite::client::{bugzilla::Client, ClientBuilder};
use bugbite::service::ServiceKind;
use tracing::info;

use crate::options::Options;

mod attachments;
mod comments;
mod get;
mod history;
mod modify;
mod search;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) fn run(
        self,
        _options: Options,
        kind: ServiceKind,
        base: String,
    ) -> anyhow::Result<ExitCode> {
        let service = kind.create(&base)?;
        info!("{service}");
        let client = ClientBuilder::new().build(service)?;
        self.cmd.run(client.into_bugzilla().unwrap())
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
    #[command(visible_alias = "h")]
    History(history::Command),
    /// Modify bugs
    #[command(visible_alias = "m")]
    Modify(modify::Command),
    /// Search bugs
    #[command(visible_alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Attachments(cmd) => cmd.run(client),
            Self::Comments(cmd) => cmd.run(client),
            Self::Get(cmd) => cmd.run(client),
            Self::History(cmd) => cmd.run(client),
            Self::Modify(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
