use std::io::{stdout, IsTerminal};
use std::process::ExitCode;

use bugbite::client::{bugzilla::Client, ClientBuilder};
use bugbite::objects::bugzilla::*;
use bugbite::service::ServiceKind;
use itertools::Itertools;
use tracing::info;

use crate::utils::truncate;

use super::{login_retry, Render};

mod attachments;
mod comments;
mod get;
mod history;
mod search;

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// Bugzilla API key
    #[arg(short = 'k', long)]
    api_key: Option<String>,

    /// Bugzilla username
    #[arg(short, long, conflicts_with = "api_key")]
    user: Option<String>,

    /// Bugzilla password
    #[arg(short, long, conflicts_with = "api_key")]
    password: Option<String>,
}

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Bugzilla")]
pub(crate) struct Command {
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
        let service = kind.create(&base)?;
        info!("{service}");
        let client = client.build(service)?.into_bugzilla().unwrap();
        Ok(login_retry(|| self.cmd.run(&client))?)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Get attachments
    #[command(visible_alias = "a")]
    Attachments(attachments::Command),
    /// Get comments
    Comments(comments::Command),
    /// Get bugs
    #[command(visible_alias = "g")]
    Get(get::Command),
    /// Get bug history
    History(history::Command),
    /// Search bugs
    #[command(visible_alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        match self {
            Self::Attachments(cmd) => cmd.run(client),
            Self::Comments(cmd) => cmd.run(client),
            Self::Get(cmd) => cmd.run(client),
            Self::History(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}

impl Render for Attachment {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        let line = format!(
            "Attachment: [{}] [{}] ({}, {}) by {}, {}",
            self.id,
            self.file_name,
            self.human_size(),
            self.content_type,
            self.creator,
            self.updated
        );

        if stdout().is_terminal() {
            writeln!(f, "{}", truncate(&line, width))
        } else {
            writeln!(f, "{line}")
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

impl Render for Event {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        if !self.changes.is_empty() {
            writeln!(f, "Changes made by {}, {}", self.who, self.when)?;
            writeln!(f, "{}", "-".repeat(width))?;
            for change in &self.changes {
                change.render(f, width)?;
            }
        }
        Ok(())
    }
}

impl Render for Change {
    fn render<W: std::io::Write>(&self, f: &mut W, _width: usize) -> std::io::Result<()> {
        let name = &self.field_name;
        match (self.removed.as_deref(), self.added.as_deref()) {
            (Some(removed), None) => write!(f, "{name}: -{removed}"),
            (Some(removed), Some(added)) => write!(f, "{name}: {removed} -> {added}"),
            (None, Some(added)) => write!(f, "{name}: +{added}"),
            (None, None) => panic!("invalid change"),
        }
    }
}

impl Render for Modification<'_> {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        match self {
            Self::Comment(comment) => comment.render(f, width),
            Self::Event(event) => event.render(f, width),
        }
    }
}

impl Render for Bug {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        if let Some(data) = self.summary.as_deref() {
            writeln!(f, "Summary: {data}")?;
        }
        if let Some(data) = self.assigned_to.as_deref() {
            writeln!(f, "Assignee: {data}")?;
        }
        if let Some(data) = self.reporter.as_deref() {
            writeln!(f, "Reporter: {data}")?;
        }
        if let Some(data) = &self.created {
            writeln!(f, "Reported: {data}")?;
        }
        if let Some(data) = &self.updated {
            writeln!(f, "Updated: {data}")?;
        }
        if let Some(data) = self.status.as_deref() {
            writeln!(f, "Status: {data}")?;
        }
        if let Some(data) = self.whiteboard.as_deref() {
            writeln!(f, "Whiteboard: {data}")?;
        }
        if let Some(data) = self.product.as_deref() {
            writeln!(f, "Product: {data}")?;
        }
        if let Some(data) = self.component.as_deref() {
            writeln!(f, "Component: {data}")?;
        }
        writeln!(f, "ID: {}", self.id)?;
        if !self.aliases.is_empty() {
            writeln!(f, "Aliases: {}", self.aliases.iter().join(", "))?;
        }
        if !self.cc.is_empty() {
            writeln!(f, "CC: {}", self.cc.iter().join(", "))?;
        }
        if !self.blocks.is_empty() {
            writeln!(f, "Blocks: {}", self.blocks.iter().join(", "))?;
        }
        if !self.depends.is_empty() {
            writeln!(f, "Depends on: {}", self.depends.iter().join(", "))?;
        }
        if !self.urls.is_empty() {
            writeln!(f, "See also: {}", self.urls.iter().join(", "))?;
        }

        // Don't count the bug description as a comment.
        if self.comments.len() > 1 {
            writeln!(f, "Comments: {}", self.comments.len() - 1)?;
        }

        if !self.attachments.is_empty() {
            writeln!(f, "Attachments: {}\n", self.attachments.len())?;
            for attachment in &self.attachments {
                attachment.render(f, width)?;
            }
        }

        for e in self.events() {
            writeln!(f)?;
            e.render(f, width)?;
        }

        Ok(())
    }
}
