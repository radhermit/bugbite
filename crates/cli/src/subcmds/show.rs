use std::io::Write;
use std::process::ExitCode;

use bugbite::config::Config;
use clap::Args;

mod connections;
mod services;

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[command(subcommand)]
    command: Subcommand,
}

impl Command {
    pub(super) fn run<W: Write>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode> {
        self.command.run(config, f)
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Show available connections
    Connections(connections::Command),
    /// Show available services
    Services(services::Command),
}

impl Subcommand {
    fn run<W: Write>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode> {
        match self {
            Self::Connections(cmd) => cmd.run(config, f),
            Self::Services(cmd) => cmd.run(config, f),
        }
    }
}
