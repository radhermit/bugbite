use std::process::ExitCode;

use bugbite::client::{github::Client, ClientBuilder};
use bugbite::service::github::Config;

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

        let client = Client::new(config, builder.build())?;
        self.cmd.run(&client).await
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
    async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(client).await,
            Self::Search(cmd) => cmd.run(client).await,
        }
    }
}
