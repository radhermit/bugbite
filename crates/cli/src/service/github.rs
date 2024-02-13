use std::process::ExitCode;

use bugbite::client::Client;
use clap::Parser;
use clap_verbosity_flag::Verbosity;

use crate::options::Options;
use crate::service::Config;

mod get;
mod modify;
mod search;

/// command line interface for github
#[derive(Debug, Parser)]
#[command(
    name = "bite-github",
    version,
    long_about = None,
    disable_help_subcommand = true,
)]
pub(crate) struct Command {
    #[command(flatten)]
    pub(super) verbosity: Verbosity,

    #[clap(flatten)]
    options: Options,

    // positional
    #[command(subcommand)]
    subcmd: Subcommand,
}

impl Command {
    pub(super) fn run(self, config: Config) -> anyhow::Result<ExitCode> {
        let client = self.options.collapse(config)?;
        self.subcmd.run(&client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Modify issues
    #[command(alias = "m")]
    Modify(modify::Command),
    /// Search issues
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        use Subcommand::*;
        match self {
            Get(cmd) => cmd.run(client),
            Modify(cmd) => cmd.run(client),
            Search(cmd) => cmd.run(client),
        }
    }
}
