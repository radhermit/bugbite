use std::process::ExitCode;

use clap::Args;

mod services;

#[derive(Debug, Args)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) fn run(self) -> anyhow::Result<ExitCode> {
        self.command.run()
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Show available services
    Services(services::Subcommand),
}

impl Subcommand {
    fn run(self) -> anyhow::Result<ExitCode> {
        match self {
            Self::Services(cmd) => cmd.run(),
        }
    }
}
