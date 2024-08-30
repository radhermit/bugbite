use std::process::ExitCode;

use bugbite::objects::github::*;
use bugbite::service::{
    github::{Config, Service},
    ClientBuilder, ServiceKind,
};
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
    pub(crate) async fn run(self, config: &crate::config::Config) -> anyhow::Result<ExitCode> {
        let connection = self.service.connection.as_str();
        let url = if ["https://", "http://"]
            .iter()
            .any(|s| connection.starts_with(s))
        {
            Ok(connection)
        } else {
            config.get_kind(ServiceKind::Github, connection)
        }?;

        let mut config = Config::new(url)?;
        config.token = self.auth.key;

        let builder = ClientBuilder::default()
            .insecure(self.service.insecure)
            .timeout(self.service.timeout);

        let service = Service::new(config, builder)?;
        debug!("Service: {service}");
        self.cmd.run(&service).await
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
    async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(service).await,
            Self::Search(cmd) => cmd.run(service).await,
        }
    }
}

impl Render<&Issue> for Service {
    fn render<W: std::io::Write>(
        &self,
        item: &Issue,
        f: &mut W,
        width: usize,
    ) -> std::io::Result<()> {
        output_field_wrapped!(f, "Title", &item.title, width);
        writeln!(f, "{:<12} : {}", "ID", item.id)?;

        Ok(())
    }
}
