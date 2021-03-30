use bugbite::client::Client;
use bugbite::config::{Config, CONFIG};
use bugbite::service::{self, ServiceKind};
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use clap::{CommandFactory, Parser};
use strum::VariantNames;

#[derive(Parser)]
#[command(disable_help_flag = true)]
pub(crate) struct Command {
    #[clap(flatten)]
    config: ConfigOpts,
    #[clap(flatten)]
    service: ServiceOpts,
}

impl Command {
    pub(crate) fn service() -> anyhow::Result<service::Config> {
        // TODO: rework partial parsing once clap supports a Parser-based API for it
        let cmd = Self::command().ignore_errors(true);
        let args = cmd.get_matches();
        let (config, connection, base, service) = (
            args.get_one::<String>("config"),
            args.get_one::<String>("connection"),
            args.get_one::<String>("base"),
            args.get_one::<ServiceKind>("service"),
        );
        let service = match (config, connection, base, service) {
            (None, Some(name), None, None) => CONFIG.get(name)?.clone(),
            (Some(path), Some(name), None, None) => {
                let config = Config::try_new(path)?;
                let service = config.get(name)?;
                service.clone()
            }
            (None, None, Some(base), Some(service)) => service.create(base)?,
            // use a stub URL so `bite -s service -h` can be used to show help output
            (None, None, None, Some(service)) => service.create("https://fake/url")?,
            // TODO: use default service from config if it exists
            _ => CONFIG.get("gentoo")?.clone(),
        };

        Ok(service)
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Config")]
#[group(requires = "connection", conflicts_with = "Service")]
pub(crate) struct ConfigOpts {
    /// use a custom config
    #[arg(long)]
    config: Option<String>,
    /// use configured connection
    #[arg(short, long)]
    connection: Option<String>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Service")]
#[group(requires_all = ["base", "service"], conflicts_with = "Config")]
pub(super) struct ServiceOpts {
    /// base service URL
    #[arg(short, long)]
    base: Option<String>,
    /// service type
    #[arg(
        short,
        long,
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
            .map(|s| s.parse::<ServiceKind>().unwrap()),
    )]
    service: Option<ServiceKind>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Connection")]
pub(crate) struct Connection {
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
pub(crate) struct Authentication {
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
