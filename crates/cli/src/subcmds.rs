use std::process::ExitCode;

use bugbite::service::ServiceKind;

use crate::options::Options;
use crate::service::*;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommand {
    // service subcommands
    Bugzilla(bugzilla::Command),
    Github(github::Command),
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
        }
    }
}
