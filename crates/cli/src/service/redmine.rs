use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::objects::redmine::*;
use bugbite::service::redmine::{self, Redmine};
use bugbite::service::ServiceKind;
use bugbite::traits::Merge;
use itertools::Itertools;
use tracing::debug;

use super::output::*;
use super::Render;

mod get;
mod search;

#[derive(clap::Args, Debug)]
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

impl From<Authentication> for redmine::Authentication {
    fn from(value: Authentication) -> Self {
        Self {
            key: value.key,
            user: value.user,
            password: value.password,
        }
    }
}

#[derive(clap::Args, Debug)]
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
        config.auth.merge(self.auth.into());
        config.client.merge(self.service.into());

        let service = config.into_service()?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Get issues
    #[command(alias = "g")]
    Get(get::Command),
    /// Search issues
    #[command(alias = "s")]
    Search(Box<search::Command>),
}

impl Subcommand {
    async fn run<W>(self, service: &Redmine, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
        }
    }
}

impl Render for Comment {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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

        if let Some(values) = &self.custom_fields {
            for field in values {
                match &field.value {
                    CustomFieldValue::String(value) => {
                        if !value.is_empty() {
                            output_field!(f, &field.name, Some(value), width);
                        }
                    }
                    CustomFieldValue::Array(values) => {
                        if !values.is_empty() {
                            wrapped_csv(f, &field.name, values, width)?;
                        }
                    }
                }
            }
        }

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
