use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::config::Config;
use bugbite::service::github::{self, Github};
use tracing::debug;

mod get;
mod search;

#[derive(clap::Args, Debug)]
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
        let service = Github::config_builder(config, self.service.connection.as_deref())?
            .auth(self.auth.into())
            .client(self.service.into())
            .build()?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Get issues
    #[command(visible_alias = "g")]
    Get(Box<get::Command>),
    /// Search issues
    #[command(visible_alias = "s")]
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run<W>(self, service: &Github, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
        }
    }
}
