use std::process::ExitCode;

use bugbite::client::{github::Client, ClientBuilder};
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
#[clap(infer_subcommands = true, next_help_heading = "GitHub")]
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
        let client = client.build(service)?.into_github().unwrap();
        self.cmd.run(client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    Get(get::Command),
    /// Search issues
    Search(search::Command),
}

impl Subcommand {
    fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}
