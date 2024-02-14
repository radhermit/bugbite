use std::process::ExitCode;

use bugbite::client::Client;
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
    pub(super) fn run(
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
        let client = Client::builder().build(service)?;
        self.cmd.run(client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Modify issues
    #[command(alias = "m")]
    Modify(modify::Command),
    /// Search issues
    #[command(alias = "s")]
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
