use std::io::stdout;
use std::process::ExitCode;

use strum::VariantNames;

use crate::config::Config;
use crate::service::*;

mod show;

#[derive(VariantNames, clap::Subcommand)]
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
        let mut stdout = stdout().lock();
        match self {
            Self::Bugzilla(cmd) => cmd.run(config, &mut stdout).await,
            Self::Github(cmd) => cmd.run(config, &mut stdout).await,
            Self::Redmine(cmd) => cmd.run(config, &mut stdout).await,
            Self::Show(cmd) => cmd.run(&mut stdout),
        }
    }
}
