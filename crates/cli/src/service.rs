use std::process::ExitCode;

use bugbite::service::{Config, ServiceKind};
use clap::Parser;
use clap_verbosity_flag::Verbosity;

mod bugzilla;
mod github;

#[allow(clippy::large_enum_variant)]
pub(super) enum Command {
    Bugzilla(bugzilla::Command),
    Github(github::Command),
}

impl Command {
    pub(super) fn parse(service: &Config) -> Self {
        match service.kind() {
            ServiceKind::BugzillaRestV1 => Self::Bugzilla(bugzilla::Command::parse()),
            ServiceKind::Github => Self::Github(github::Command::parse()),
        }
    }

    pub(super) fn verbosity(&self) -> &Verbosity {
        match self {
            Self::Bugzilla(cmd) => &cmd.verbosity,
            Self::Github(cmd) => &cmd.verbosity,
        }
    }

    pub(super) fn run(self, service: Config) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(service),
            Self::Github(cmd) => cmd.run(service),
        }
    }
}
