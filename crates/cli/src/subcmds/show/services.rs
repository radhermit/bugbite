use std::io::{self, Write};
use std::process::ExitCode;

use bugbite::service::ServiceKind;
use bugbite::services::SERVICES;
use clap::Args;
use indexmap::IndexMap;
use itertools::Itertools;

#[derive(Debug, Args)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run(self) -> anyhow::Result<ExitCode> {
        let mut stdout = io::stdout().lock();
        let mut services = IndexMap::<ServiceKind, Vec<(&str, &str)>>::new();
        for (name, config) in SERVICES.iter() {
            services
                .entry(config.kind())
                .or_default()
                .push((name, config.base().as_str()));
        }

        for (kind, entries) in services.iter().sorted() {
            writeln!(stdout, "Service: {kind}")?;
            for (name, base) in entries.iter().sorted() {
                writeln!(stdout, "  {name:<12}: {}", base)?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
