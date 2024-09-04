use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::objects::redmine::*;
use bugbite::service::{redmine::Service, ServiceKind};
use bugbite::traits::MergeOption;
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
    #[arg(short, long, env = "BUGBITE_PASS")]
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
    pub(crate) async fn run<W>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        // load or create a service config
        let connection = self.service.connection.as_str();
        let mut config = config
            .get_kind(ServiceKind::Redmine, connection)?
            .into_redmine()
            .map_err(|_| anyhow!("incompatible connection: {connection}"))?;

        // cli options override config settings
        config.key = config.key.merge(self.auth.key);
        config.user = config.user.merge(self.auth.user);
        config.password = config.password.merge(self.auth.password);
        config.client.certificate = config.client.certificate.merge(self.service.certificate);
        config.client.insecure = config.client.insecure.merge(self.service.insecure);
        config.client.timeout = config.client.timeout.merge(self.service.timeout);

        let service = Service::from_config(config)?;
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
