use std::process::ExitCode;

use bugbite::objects::redmine::*;
use bugbite::service::{
    redmine::{Config, Service},
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
    /// API key
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
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        match self {
            Self::Get(cmd) => cmd.run(service).await,
            Self::Search(cmd) => cmd.run(service).await,
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
        writeln!(f, "by {}, {}", self.user, self.created)?;
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
        output_field!(f, "Reporter", &self.author, width);
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
