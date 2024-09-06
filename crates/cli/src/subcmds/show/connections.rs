use std::io::Write;
use std::process::ExitCode;

use bugbite::config::Config;
use clap::Args;
use itertools::Itertools;

#[derive(Args, Debug)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run<W: Write>(&self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode> {
        let connections: Vec<_> = config.keys().sorted().collect();

        for name in &connections {
            writeln!(f, "{name}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
