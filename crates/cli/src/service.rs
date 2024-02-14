use std::process::ExitCode;

use bugbite::client::Client;

pub(crate) mod bugzilla;
pub(crate) mod github;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Subcommand {
    Bugzilla(bugzilla::Command),
    Github(github::Command),
}

impl Subcommand {
    pub(crate) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Bugzilla(cmd) => cmd.run(client),
            Self::Github(cmd) => cmd.run(client),
        }
    }
}
