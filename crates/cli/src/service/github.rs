use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::objects::github::*;
use bugbite::service::github::{self, Service};
use bugbite::service::ServiceKind;
use bugbite::traits::Merge;
use itertools::Itertools;
use tracing::debug;

use super::output::*;
use super::Render;

mod get;
mod search;

#[derive(clap::Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// GitHub personal access token
    #[arg(short, long, env = "BUGBITE_KEY")]
    key: Option<String>,

    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,
}

impl From<Authentication> for github::Authentication {
    fn from(value: Authentication) -> Self {
        Self {
            user: value.user,
            token: value.key,
        }
    }
}

#[derive(clap::Args)]
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
        let connection = self.service.connection.as_str();
        let mut config = config
            .get_kind(ServiceKind::Github, connection)?
            .into_github()
            .map_err(|_| anyhow!("incompatible connection: {connection}"))?;

        // cli options override config settings
        config.auth.merge(self.auth.into());
        config.client.merge(self.service.into());

        let service = Service::from_config(config)?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Search issues
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
        }
    }
}

impl Render<&Issue> for Service {
    fn render<W>(&self, item: &Issue, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        output_field_wrapped!(f, "Title", &item.title, width);
        writeln!(f, "{:<12} : {}", "ID", item.id)?;

        Ok(())
    }
}
