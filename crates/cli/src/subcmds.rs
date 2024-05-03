use std::process::ExitCode;

use strum::VariantNames;

use crate::config::Config;
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
    pub(crate) async fn run(self, config: &Config) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(config).await,
            Self::Github(cmd) => cmd.run(config).await,
            Self::Redmine(cmd) => cmd.run(config).await,
            Self::Show(cmd) => cmd.run(),
        }
    }
}
