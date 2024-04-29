use std::process::ExitCode;

use bugbite::service::ClientBuilder;
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
    /// redmine service support
    Redmine(redmine::Command),

    // regular subcommands
    /// show various bite-related information
    Show(show::Command),
}

impl Subcommand {
    pub(crate) async fn run(self, base: String, client: ClientBuilder) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(base, client).await,
            Self::Github(cmd) => cmd.run(base, client).await,
            Self::Redmine(cmd) => cmd.run(base, client).await,
            Self::Show(cmd) => cmd.run(),
        }
    }
}
