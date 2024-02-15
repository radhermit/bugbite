use std::io::{self, Write};
use std::process::ExitCode;

use clap::Args;

use bugbite::services::SERVICES;

#[derive(Debug, Args)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run(self) -> anyhow::Result<ExitCode> {
        let mut stdout = io::stdout().lock();
        for (name, config) in SERVICES.iter() {
            writeln!(stdout, "{name}: {}", config.base())?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
