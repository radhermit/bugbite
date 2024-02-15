use std::process::ExitCode;

use bugbite::client::{github::Client, ClientBuilder};
use bugbite::service::ServiceKind;
use tracing::info;

use crate::options::Options;

mod get;
mod modify;
mod search;

#[derive(Debug, clap::Args)]
pub(crate) struct Command {
    /// project to target
    #[arg(short, long)]
    project: Option<String>,
    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) fn run(
        self,
        _options: Options,
        kind: ServiceKind,
        base: String,
    ) -> anyhow::Result<ExitCode> {
        let service = match self.project {
            Some(project) => kind.create(&format!("https://github.com/{project}"))?,
            None => kind.create(&base)?,
        };
        info!("{service}");
        let client = ClientBuilder::new().build(service)?;
        self.cmd.run(client.into_github().unwrap())
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(visible_alias = "g")]
    Get(get::Command),
    /// Modify issues
    #[command(visible_alias = "m")]
    Modify(modify::Command),
    /// Search issues
    #[command(visible_alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(client),
            Self::Modify(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
