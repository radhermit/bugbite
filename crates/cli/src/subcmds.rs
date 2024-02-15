use std::process::ExitCode;

use bugbite::service::ServiceKind;
use strum::VariantNames;

use crate::options::Options;
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
    pub(crate) fn run(
        self,
        options: Options,
        kind: ServiceKind,
        base: String,
    ) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(options, kind, base),
            Self::Github(cmd) => cmd.run(options, kind, base),
            Self::Show(cmd) => cmd.run(),
        }
    }
}
