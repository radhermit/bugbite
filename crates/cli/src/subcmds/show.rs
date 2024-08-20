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
    pub(super) fn run(self) -> anyhow::Result<ExitCode> {
        self.command.run()
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
    fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Connections(cmd) => cmd.run(),
            Self::Services(cmd) => cmd.run(),
        }
    }
}
