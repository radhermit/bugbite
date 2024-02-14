use bugbite::client::Client;
use bugbite::config::{Config, CONFIG};
use bugbite::service::{self, ServiceKind};
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use clap::{Parser, Subcommand};
use strum::VariantNames;

#[derive(Parser)]
#[command(disable_help_flag = true)]
pub(crate) struct Command {
    #[clap(flatten)]
    config_opts: ConfigOpts,
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
    pub(crate) fn service() -> anyhow::Result<service::Config> {
        let service = if let Ok(cmd) = Command::try_parse() {
            let config = cmd.config_opts.config;
            let connection = cmd.config_opts.connection;
            let base = cmd.service_opts.base;
            let service = cmd.service_opts.service;
            match (config, connection, base, service) {
                (None, Some(name), None, None) => CONFIG.get(&name)?.clone(),
                (Some(path), Some(name), None, None) => {
                    let config = Config::try_new(path)?;
                    let service = config.get(&name)?;
                    service.clone()
                }
                (None, None, Some(base), Some(service)) => service.create(&base)?,
                // use a stub URL so `bite -s service -h` can be used to show help output
                (None, None, None, Some(service)) => service.create("https://fake/url")?,
                // TODO: use default service from config if it exists
                _ => CONFIG.get("gentoo")?.clone(),
            }
        } else {
            CONFIG.get("gentoo")?.clone()
        };

        Ok(service)
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Config")]
#[group(requires = "connection", conflicts_with = "ServiceOpts")]
struct ConfigOpts {
    /// use a custom config
    #[arg(long)]
    config: Option<String>,
    /// use configured connection
    #[arg(short, long, env = "BUGBITE_CONNECTION")]
    connection: Option<String>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Service")]
#[group(requires_all = ["base", "service"], conflicts_with = "ConfigOpts")]
struct ServiceOpts {
    /// base service URL
    #[arg(short, long, env = "BUGBITE_BASE")]
    base: Option<String>,
    /// service type
    #[arg(
        short,
        long,
        env = "BUGBITE_SERVICE",
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
    _config: ConfigOpts,
    #[clap(flatten)]
    _service: ServiceOpts,
    #[clap(flatten)]
    connection: Connection,
    #[clap(flatten)]
    auth: Authentication,
}

impl Options {
    pub(super) fn collapse(self, service: service::Config) -> anyhow::Result<Client> {
        let client = Client::builder().build(service)?;
        Ok(client)
    }
}
