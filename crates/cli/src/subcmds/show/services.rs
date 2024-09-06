use std::io::Write;
use std::process::ExitCode;

use bugbite::config::Config;
use bugbite::service::ServiceKind;
use clap::Args;
use indexmap::IndexMap;
use itertools::Itertools;

#[derive(Args, Debug)]
pub(super) struct Subcommand {}

impl Subcommand {
    pub(super) fn run<W: Write>(&self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode> {
        let mut services = IndexMap::<ServiceKind, Vec<(&str, &str)>>::new();
        for (name, config) in config {
            services
                .entry(config.kind())
                .or_default()
                .push((name, config.base().as_str()));
        }

        for (kind, entries) in services.iter().sorted() {
            writeln!(f, "Service: {kind}")?;
            for (name, base) in entries.iter().sorted() {
                writeln!(f, "  {name:<12}: {}", base)?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
