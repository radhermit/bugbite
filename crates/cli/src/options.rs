use bugbite::client::Client;
use bugbite::service::{Config, ServiceKind};
use bugbite::services::SERVICES;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use clap::{Parser, Subcommand};
use strum::VariantNames;

#[derive(Parser)]
#[command(disable_help_flag = true)]
pub(crate) struct Command {
    #[clap(flatten)]
    service_opts: ServiceOpts,
    #[command(subcommand)]
    subcmd: Remaining,
}

#[derive(Subcommand)]
enum Remaining {
    #[command(external_subcommand)]
    Remaining(Vec<String>),
}

impl Command {
    pub(crate) fn service() -> anyhow::Result<Config> {
        let service = if let Ok(cmd) = Command::try_parse() {
            let connection = cmd.service_opts.connection;
            let base = cmd.service_opts.base;
            let service = cmd.service_opts.service;
            match (connection, base, service) {
                (Some(name), None, None) => SERVICES.get(&name)?.clone(),
                (None, Some(base), Some(service)) => service.create(&base)?,
                // use a stub URL so `bite -s service -h` can be used to show help output
                (None, None, Some(service)) => service.create("https://fake/url")?,
                // TODO: use default service from config if it exists
                _ => SERVICES.get("gentoo")?.clone(),
            }
        } else {
            SERVICES.get("gentoo")?.clone()
        };

        Ok(service)
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
    #[arg(short, long, env = "BUGBITE_BASE", requires = "service")]
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

impl Options {
    pub(super) fn collapse(self, service: Config) -> anyhow::Result<Client> {
        let client = Client::builder().build(service)?;
        Ok(client)
    }
}
