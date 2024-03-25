use std::process::ExitCode;

use bugbite::client::{redmine::Client, ClientBuilder};
use bugbite::objects::redmine::*;
use bugbite::service::redmine::Config;
use itertools::Itertools;

use super::output::*;
use super::Render;

mod get;
mod search;

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// key or token
    #[arg(short, long, env = "BUGBITE_KEY")]
    key: Option<String>,

    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,

    /// password
    #[arg(short, long, requires = "user", env = "BUGBITE_PASS")]
    password: Option<String>,
}

#[derive(Debug, clap::Args)]
#[clap(infer_subcommands = true, next_help_heading = "Redmine")]
pub(crate) struct Command {
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
        let mut config = Config::new(&base)?;
        config.key = self.auth.key;
        config.user = self.auth.user;
        config.password = self.auth.password;

        let client = Client::new(config, builder.build())?;
        self.cmd.run(&client).await
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get issues
    Get(get::Command),
    /// Search issues
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(client).await,
            Self::Search(cmd) => cmd.run(client).await,
        }
    }
}

impl Render for Comment {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        if self.count != 0 {
            write!(f, "Comment #{} ", self.count)?;
        } else {
            write!(f, "Description ")?;
        }
        writeln!(f, "by {}, {}", self.creator, self.created)?;
        writeln!(f, "{}", "-".repeat(width))?;
        // wrap comment text
        let wrapped = textwrap::wrap(self.text.trim(), width);
        writeln!(f, "{}", wrapped.iter().join("\n"))
    }
}

impl Render for Issue {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        output_field_wrapped!(f, "Subject", &self.subject, width);
        output_field!(f, "Assignee", &self.assigned_to, width);
        output_field!(f, "Reporter", &self.creator, width);
        output_field!(f, "Status", &self.status, width);
        output_field!(f, "Tracker", &self.tracker, width);
        output_field!(f, "Priority", &self.priority, width);
        output_field!(f, "Closed", &self.closed, width);
        output_field!(f, "Created", &self.created, width);
        output_field!(f, "Updated", &self.updated, width);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;

        if !self.comments.is_empty() {
            writeln!(f, "{:<12} : {}", "Comments", self.comments.len())?;
        }

        // render both comments
        for comment in &self.comments {
            writeln!(f)?;
            comment.render(f, width)?;
        }

        Ok(())
    }
}
