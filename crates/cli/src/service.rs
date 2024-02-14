use std::process::ExitCode;

use bugbite::service::ServiceKind;

use crate::options::Options;

pub(crate) mod bugzilla;
pub(crate) mod github;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommand {
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
