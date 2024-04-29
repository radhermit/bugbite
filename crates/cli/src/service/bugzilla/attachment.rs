use std::process::ExitCode;

use bugbite::service::bugzilla::Service;

mod create;
mod get;
mod update;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        self.command.run(service).await
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

    /// Update attachments
    #[command(alias = "u")]
    Update(update::Command),
}

impl Subcommand {
    async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        match self {
            Self::Create(cmd) => cmd.run(service).await,
            Self::Get(cmd) => cmd.run(service).await,
            Self::Update(cmd) => cmd.run(service).await,
        }
    }
}
