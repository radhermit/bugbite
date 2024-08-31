use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Service;

mod create;
mod get;
mod update;

#[derive(clap::Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        self.command.run(service, f).await
    }
}

#[derive(clap::Subcommand)]
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
    async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Create(cmd) => cmd.run(service, f).await,
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Update(cmd) => cmd.run(service, f).await,
        }
    }
}
