use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Bugzilla;

mod get;
mod tag;

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
    /// Get comments
    #[command(visible_alias = "g")]
    Get(get::Command),

    /// tag comments
    #[command(visible_alias = "t")]
    Tag(tag::Command),
}

impl Subcommand {
    async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Tag(cmd) => cmd.run(service, f).await,
        }
    }
}
