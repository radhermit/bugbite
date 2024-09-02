use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::objects::redmine::*;
use bugbite::service::{
    redmine::{Config, Service},
    ServiceKind,
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
    pub(crate) async fn run<W>(
        self,
        config: &crate::config::Config,
        f: &mut W,
    ) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let connection = self.service.connection.as_str();
        let url = if ["https://", "http://"]
            .iter()
            .any(|s| connection.starts_with(s))
        {
            Ok(connection)
        } else {
            config.get_kind(ServiceKind::Redmine, connection)
        }?;

        let mut config = Config::new(url)?;
        config.key = self.auth.key;
        config.user = self.auth.user;
        config.password = self.auth.password;

        let builder = self.service.into();
        let service = Service::new(config, builder)?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Search issues
    #[command(alias = "s")]
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
        }
    }
}

impl Render<&Comment> for Service {
    fn render<W>(&self, item: &Comment, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        if item.count != 0 {
            write!(f, "Comment #{} ", item.count)?;
        } else {
            write!(f, "Description ")?;
        }
        writeln!(f, "by {}, {}", item.user, item.created)?;
        writeln!(f, "{}", "-".repeat(width))?;
        // wrap comment text
        let wrapped = textwrap::wrap(item.text.trim(), width);
        writeln!(f, "{}", wrapped.iter().join("\n"))
    }
}

impl Render<&Issue> for Service {
    fn render<W>(&self, item: &Issue, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        output_field_wrapped!(f, "Subject", &item.subject, width);
        output_field!(f, "Assignee", &item.assigned_to, width);
        output_field!(f, "Reporter", &item.author, width);
        output_field!(f, "Status", &item.status, width);
        output_field!(f, "Tracker", &item.tracker, width);
        output_field!(f, "Priority", &item.priority, width);
        output_field!(f, "Closed", &item.closed, width);
        output_field!(f, "Created", &item.created, width);
        output_field!(f, "Updated", &item.updated, width);
        writeln!(f, "{:<12} : {}", "ID", item.id)?;

        if !item.comments.is_empty() {
            writeln!(f, "{:<12} : {}", "Comments", item.comments.len())?;
        }

        // render both comments
        for comment in &item.comments {
            writeln!(f)?;
            self.render(comment, f, width)?;
        }

        Ok(())
    }
}
