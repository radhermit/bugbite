use std::process::ExitCode;

use bugbite::client::bugzilla::Client;

mod create;
mod get;
mod update;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        self.command.run(client).await
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Create attachments
    #[command(alias = "c")]
    Create(create::Command),

    /// Get attachments
    #[command(alias = "g")]
    Get(get::Command),

    /// Update attachment metadata
    #[command(alias = "u")]
    Update(update::Command),
}

impl Subcommand {
    async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run(client).await,
            Self::Get(cmd) => cmd.run(client).await,
            Self::Update(cmd) => cmd.run(client).await,
        }
    }
}
