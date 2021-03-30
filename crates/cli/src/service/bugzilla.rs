use std::process::ExitCode;

use bugbite::client::Client;
use clap::Parser;
use clap_verbosity_flag::Verbosity;

use crate::options::Options;
use crate::service::Config;

mod attachments;
mod get;
mod modify;
mod search;

/// command line interface for bugzilla
#[derive(Debug, Parser)]
#[command(name = "bite", version, long_about = None, disable_help_subcommand = true)]
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

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get attachments from bugs
    #[command(alias = "a")]
    Attachments(attachments::Command),
    /// Get bugs
    #[command(alias = "g")]
    Get(get::Command),
    /// Modify bugs
    #[command(alias = "m")]
    Modify(modify::Command),
    /// Search bugs
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        use Subcommand::*;
        match self {
            Attachments(cmd) => cmd.run(client),
            Get(cmd) => cmd.run(client),
            Modify(cmd) => cmd.run(client),
            Search(cmd) => cmd.run(client),
        }
    }
}
