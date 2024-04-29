use std::process::ExitCode;

use bugbite::objects::github::*;
use bugbite::service::{
    github::{Config, Service},
    ClientBuilder,
};
use itertools::Itertools;
use tracing::info;

use super::output::*;
use super::Render;

mod get;
mod search;

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// GitHub personal access token
    #[arg(short, long)]
    token: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// project to target
    #[arg(short, long)]
    project: Option<String>,

    #[clap(flatten)]
    auth: Authentication,

    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) async fn run(
        self,
        base: String,
        builder: ClientBuilder,
    ) -> anyhow::Result<ExitCode> {
        let base = match self.project.as_ref() {
            Some(project) => format!("https://github.com/{project}"),
            None => base,
        };

        let mut config = Config::new(&base)?;
        config.token = self.auth.token;

        let service = Service::new(config, builder)?;
        info!("Service: {service}");
        self.cmd.run(&service).await
    }
}

#[derive(Debug, clap::Subcommand)]
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

impl Render for Issue {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        output_field_wrapped!(f, "Title", &self.title, width);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;

        Ok(())
    }
}
