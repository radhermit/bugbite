use std::io::Write;
use std::process::ExitCode;

use clap::Args;

mod connections;
mod services;

#[derive(Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) fn run<W: Write>(self, f: &mut W) -> anyhow::Result<ExitCode> {
        self.command.run(f)
    }
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Show available connections
    Connections(connections::Subcommand),
    /// Show available services
    Services(services::Subcommand),
}

impl Subcommand {
    fn run<W: Write>(self, f: &mut W) -> anyhow::Result<ExitCode> {
        match self {
            Self::Connections(cmd) => cmd.run(f),
            Self::Services(cmd) => cmd.run(f),
        }
    }
}
