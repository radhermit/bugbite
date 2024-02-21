use std::process::ExitCode;

use bugbite::client::ClientBuilder;
use strum::VariantNames;

use crate::service::*;

mod show;

#[derive(Debug, VariantNames, clap::Subcommand)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum Subcommand {
    // service subcommands
    /// bugzilla service support
    Bugzilla(bugzilla::Command),
    /// github service support
    Github(github::Command),

    // regular subcommands
    /// show various bite-related information
    Show(show::Command),
}

impl Subcommand {
    pub(crate) fn run(self, base: String, client: ClientBuilder) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(base, client),
            Self::Github(cmd) => cmd.run(base, client),
            Self::Show(cmd) => cmd.run(),
        }
    }
}
