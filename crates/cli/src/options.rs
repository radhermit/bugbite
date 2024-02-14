use std::collections::HashMap;
use std::io::stderr;
use std::process::ExitCode;

use bugbite::service::{Config, ServiceKind};
use bugbite::services::SERVICES;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use strum::{IntoEnumIterator, VariantNames};
use tracing::info;
use tracing_log::AsTrace;

use crate::utils::COLUMNS;

#[derive(Debug, Parser)]
#[command(disable_help_flag = true)]
pub(super) struct ServiceCommand {
    #[command(flatten)]
    _verbosity: Verbosity,
    #[clap(flatten)]
    service_opts: ServiceOpts,
    #[command(subcommand)]
    subcmd: Remaining,
}

#[derive(Debug, Subcommand)]
enum Remaining {
    #[command(external_subcommand)]
    Args(Vec<String>),
}

impl ServiceCommand {
    pub(crate) fn service() -> anyhow::Result<(Config, Vec<String>)> {
        // create mapping for service kind to subcommand names
        let subcmds: HashMap<ServiceKind, String> = ServiceKind::iter()
            .map(|x| match x.as_ref().split_once('-') {
                Some(vals) => (x, vals.0.into()),
                None => (x, x.to_string()),
            })
            .collect();

        // parse connection info
        let Ok(cmd) = Self::try_parse() else {
            // fallback for `bite -h/--help` usage
            Command::parse();
            panic!("command parsing should have exited");
        };

        let connection = &cmd.service_opts.connection;
        let base = &cmd.service_opts.base;
        let service = &cmd.service_opts.service;
        let Remaining::Args(remaining) = cmd.subcmd;
        let subcmd = remaining.first().map(|s| s.as_str()).unwrap_or_default();
        let subcmd_kind = subcmds
            .iter()
            .find(|(_, v)| v.as_str() == subcmd)
            .map(|(k, _)| k);

        // determine service type
        let service = match (connection, base, service) {
            (Some(name), None, None) => SERVICES.get(name)?.clone(),
            (None, Some(base), Some(service)) => service.create(base)?,
            (None, Some(base), None) => ServiceKind::default().create(base)?,
            (None, None, None) => match subcmd_kind {
                Some(kind) => {
                    // support help output for usage such as `bite bugzilla -h`
                    Command::parse();
                    // TODO: add mapping for default services for each type?
                    anyhow::bail!("no default service: {kind}");
                }
                // TODO: use default service from config if it exists
                None => SERVICES.get("gentoo")?.clone(),
            },
            _ => panic!("misconfigured service option restrictions"),
        };

        // construct new args for the main command to parse
        let mut args = vec![env!("CARGO_BIN_NAME").to_string()];

        // inject subcommand for requested service type if missing
        if let Some(kind) = subcmd_kind {
            let selected = &service.kind();
            if kind != selected {
                anyhow::bail!("{subcmd} not compatible with service: {selected}");
            }
        } else {
            let cmd = subcmds.get(&service.kind()).unwrap();
            args.push(cmd.clone());
        }

        args.extend(remaining);
        Ok((service, args))
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Service")]
struct ServiceOpts {
    /// use pre-configured connection
    #[arg(
        short,
        long,
        env = "BUGBITE_CONNECTION",
        conflicts_with_all = ["base", "service"]
    )]
    connection: Option<String>,
    /// base service URL
    #[arg(short, long, env = "BUGBITE_BASE")]
    base: Option<String>,
    /// service type
    #[arg(
        short,
        long,
        env = "BUGBITE_SERVICE",
        requires = "base",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
            .map(|s| s.parse::<ServiceKind>().unwrap()),
    )]
    service: Option<ServiceKind>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Connection")]
struct Connection {
    /// skip SSL certificate verification
    #[arg(short, long)]
    insecure: bool,
    /// max number of concurrent requests
    #[arg(long)]
    concurrent: Option<usize>,
    /// seconds to wait before request timeout
    #[arg(long)]
    timeout: Option<usize>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// skip service authentication
    #[arg(short = 'S', long)]
    skip: bool,
    /// username
    #[arg(short, long)]
    user: Option<String>,
    /// password
    #[arg(short, long)]
    password: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[clap(flatten)]
    _service: ServiceOpts,
    #[clap(flatten)]
    connection: Connection,
    #[clap(flatten)]
    auth: Authentication,
}

/// command line interface for bugbite
#[derive(Debug, Parser)]
#[command(
    name = "bite",
    version,
    long_about = None,
    disable_help_subcommand = true,
    term_width = *COLUMNS,
)]
pub(crate) struct Command {
    #[command(flatten)]
    verbosity: Verbosity,

    #[clap(flatten)]
    options: Options,

    #[command(subcommand)]
    subcmd: crate::service::Subcommand,
}

impl Command {
    pub(super) fn run(self, service: Config) -> anyhow::Result<ExitCode> {
        // custom log event formatter
        let format = tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(false)
            .without_time()
            .compact();

        tracing_subscriber::fmt()
            .event_format(format)
            .with_max_level(self.verbosity.log_level_filter().as_trace())
            .with_writer(stderr)
            .init();

        info!("{service}");
        self.subcmd.run(self.options, service)
    }
}
