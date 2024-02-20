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

        writeln!(f, "{}", truncate(&line, width))
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
            (Some(removed), None) => writeln!(f, "{name}: -{removed}"),
            (Some(removed), Some(added)) => writeln!(f, "{name}: {removed} -> {added}"),
            (None, Some(added)) => writeln!(f, "{name}: +{added}"),
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

/// Output an iterable field in wrapped CSV format.
fn wrapped_csv<W, S>(f: &mut W, name: &str, data: &[S], width: usize) -> std::io::Result<()>
where
    W: std::io::Write,
    S: std::fmt::Display,
{
    if !data.is_empty() {
        let rendered = data.iter().join(", ");
        if rendered.len() + name.len() + 2 <= width {
            writeln!(f, "{name}: {rendered}")?;
        } else {
            let options = textwrap::Options::new(width)
                .initial_indent("  ")
                .subsequent_indent("  ");
            let wrapped = textwrap::wrap(&rendered, &options);
            writeln!(f, "{name}:\n{}", wrapped.iter().join("\n"))?;
        }
    }
    Ok(())
}

/// Output an iterable field in truncated list format.
fn truncated_list<W, S>(f: &mut W, name: &str, data: &[S], width: usize) -> std::io::Result<()>
where
    W: std::io::Write,
    S: AsRef<str>,
{
    match data {
        [] => Ok(()),
        [value] => writeln!(f, "{name}: {}", truncate(value.as_ref(), width)),
        values => {
            let list = values
                .iter()
                .map(|s| truncate(s.as_ref(), width - 2))
                .join("\n  ");
            writeln!(f, "{name}:\n  {list}")
        }
    }
}

impl Render for Bug {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        if let Some(value) = &self.summary {
            writeln!(f, "Summary: {value}")?;
        }
        if let Some(value) = &self.assigned_to {
            writeln!(f, "Assignee: {value}")?;
        }
        if let Some(value) = &self.reporter {
            writeln!(f, "Reporter: {value}")?;
        }
        if let Some(value) = &self.created {
            writeln!(f, "Reported: {value}")?;
        }
        if let Some(value) = &self.updated {
            writeln!(f, "Updated: {value}")?;
        }
        if let Some(value) = &self.status {
            writeln!(f, "Status: {value}")?;
        }
        if let Some(value) = &self.resolution {
            writeln!(f, "Resolution: {value}")?;
        }
        if let Some(value) = &self.duplicate_of {
            writeln!(f, "Duplicate of: {value}")?;
        }
        if let Some(value) = &self.whiteboard {
            writeln!(f, "Whiteboard: {value}")?;
        }
        if let Some(value) = &self.product {
            writeln!(f, "Product: {value}")?;
        }
        if let Some(value) = &self.component {
            writeln!(f, "Component: {value}")?;
        }
        writeln!(f, "ID: {}", self.id)?;
        wrapped_csv(f, "Aliases", &self.aliases, width)?;
        wrapped_csv(f, "CC", &self.cc, width)?;
        wrapped_csv(f, "Blocks", &self.blocks, width)?;
        wrapped_csv(f, "Depends on", &self.depends, width)?;
        if !self.urls.is_empty() {
            truncated_list(f, "See also", &self.urls, width)?;
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

        // render both comments and changes in order of occurrence if either exist
        for e in self.events() {
            writeln!(f)?;
            e.render(f, width)?;
        }

        Ok(())
    }
}
