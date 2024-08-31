use std::io::Write;
use std::process::ExitCode;

use bugbite::services::SERVICES;
use clap::Args;
use itertools::Itertools;

#[derive(Args)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run<W: Write>(self, f: &mut W) -> anyhow::Result<ExitCode> {
        let connections: Vec<_> = SERVICES.keys().sorted().collect();

        for name in &connections {
            writeln!(f, "{name}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
