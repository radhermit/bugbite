use std::io::{self, Write};
use std::process::ExitCode;

use bugbite::services::SERVICES;
use clap::Args;
use itertools::Itertools;

#[derive(Args)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run(self) -> anyhow::Result<ExitCode> {
        let connections: Vec<_> = SERVICES.keys().sorted().collect();

        let mut stdout = io::stdout().lock();
        for name in &connections {
            writeln!(stdout, "{name}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
