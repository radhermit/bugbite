use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Bugzilla;

mod create;
mod get;
mod update;

#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        self.command.run(service, f).await
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Create attachments
    #[command(visible_alias = "c")]
    Create(create::Command),

    /// Get attachments
    #[command(visible_alias = "g")]
    Get(get::Command),

    /// Update attachments
    #[command(visible_alias = "u")]
    Update(update::Command),
}

impl Subcommand {
    async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
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
