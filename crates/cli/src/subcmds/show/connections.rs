use std::collections::HashSet;
use std::io::Write;
use std::process::ExitCode;

use bugbite::config::Config;
use bugbite::service::ServiceKind;
use bugbite::traits::WebClient;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::{IntoEnumIterator, VariantNames};

#[derive(Args, Debug)]
pub(super) struct Command {
    /// service types
    #[clap(
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
            .map(|s| s.parse::<ServiceKind>().unwrap()),
        help_heading = "Arguments",
    )]
    service: Vec<ServiceKind>,
}

impl Command {
    pub(super) fn run<W: Write>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode> {
        let services: HashSet<_> = if self.service.is_empty() {
            ServiceKind::iter().collect()
        } else {
            self.service.into_iter().collect()
        };

        for (name, config) in &config.services {
            if services.contains(&config.kind()) {
                writeln!(f, "{name}")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
