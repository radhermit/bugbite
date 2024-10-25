use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::config::Config;
use bugbite::service::redmine::{self, Redmine};
use bugbite::service::ServiceKind;
use bugbite::traits::Merge;
use tracing::debug;

mod get;
mod search;

#[derive(clap::Args, Debug)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// API key
    #[arg(short, long, env = "BUGBITE_KEY")]
    key: Option<String>,

    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,

    /// password
    #[arg(short, long, env = "BUGBITE_PASS")]
    password: Option<String>,
}

impl From<Authentication> for redmine::Authentication {
    fn from(value: Authentication) -> Self {
        Self {
            key: value.key,
            user: value.user,
            password: value.password,
        }
    }
}

#[derive(clap::Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    service: super::ServiceOptions,

    #[clap(flatten)]
    auth: Authentication,

    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) async fn run<W>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        // load or create a service config
        let connection = self.service.connection(config)?;
        let mut config = config
            .get_kind(ServiceKind::Redmine, connection)?
            .into_redmine()
            .unwrap();

        // cli options override config settings
        config.auth.merge(self.auth.into());
        config.client.merge(self.service.into());

        let service = config.into_service()?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Get issues
    #[command(visible_alias = "g")]
    Get(get::Command),
    /// Search issues
    #[command(visible_alias = "s")]
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run<W>(self, service: &Redmine, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
        }
    }
}
