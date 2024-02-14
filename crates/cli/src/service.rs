use std::process::ExitCode;

use bugbite::service;

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
        service: service::Config,
    ) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(options, service),
            Self::Github(cmd) => cmd.run(options, service),
        }
    }
}
