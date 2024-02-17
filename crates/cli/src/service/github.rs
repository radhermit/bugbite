use std::process::ExitCode;

use bugbite::client::{Client, ClientBuilder};
use bugbite::service::ServiceKind;
use tracing::info;

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
    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) fn run(
        self,
        kind: ServiceKind,
        base: String,
        client: ClientBuilder,
    ) -> anyhow::Result<ExitCode> {
        let service = match self.project {
            Some(project) => kind.create(&format!("https://github.com/{project}"))?,
            None => kind.create(&base)?,
        };
        info!("{service}");
        self.cmd.run(client.build(service)?)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(visible_alias = "g")]
    Get(get::Command),
    /// Search issues
    #[command(visible_alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let client = client.into_github().unwrap();
        match self {
            Self::Get(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
