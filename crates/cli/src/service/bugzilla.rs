use std::process::ExitCode;

use bugbite::client::{bugzilla::Client, ClientBuilder};
use bugbite::objects::bugzilla::*;
use bugbite::service::bugzilla::Config;
use itertools::Itertools;

use crate::utils::truncate;

use super::output::*;
use super::Render;

mod attach;
mod attachment;
mod comment;
mod create;
mod get;
mod history;
mod modify;
mod search;

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// key or token
    #[arg(short = 'k', long, env = "BUGBITE_KEY")]
    key: Option<String>,

    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,

    /// password
    #[arg(short, long, requires = "user", env = "BUGBITE_PASS")]
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
    pub(crate) fn run(self, base: String, builder: ClientBuilder) -> anyhow::Result<ExitCode> {
        let mut config = Config::new(&base)?;
        config.key = self.auth.key;
        config.user = self.auth.user;
        config.password = self.auth.password;

        let client = Client::new(config, builder.build())?;
        self.cmd.run(&client)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Create attachments
    #[command(alias = "at")]
    Attach(attach::Command),
    /// Get attachments
    #[command(alias = "a")]
    Attachment(attachment::Command),
    /// Get comments
    Comment(comment::Command),
    /// Create bug
    #[command(alias = "c")]
    Create(Box<create::Command>),
    /// Get bugs
    #[command(alias = "g")]
    Get(get::Command),
    /// Get changes
    History(history::Command),
    /// Modify bugs
    #[command(alias = "m")]
    Modify(Box<modify::Command>),
    /// Search bugs
    #[command(alias = "s")]
    Search(Box<search::Command>),
}

impl Subcommand {
    fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        match self {
            Self::Attach(cmd) => cmd.run(client),
            Self::Attachment(cmd) => cmd.run(client),
            Self::Comment(cmd) => cmd.run(client),
            Self::Create(cmd) => cmd.run(client),
            Self::Get(cmd) => cmd.run(client),
            Self::History(cmd) => cmd.run(client),
            Self::Modify(cmd) => cmd.run(client),
            Self::Search(cmd) => cmd.run(client),
        }
    }
}

impl Render for Attachment {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        let line = format!(
            "{}: [{}] ({}, {}) by {}, {}",
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
            write!(f, "Comment #{}", self.count)?;
        } else {
            write!(f, "Description")?;
        }
        if !self.tags.is_empty() {
            write!(f, " ({})", self.tags.iter().join(", "))?;
        }
        if self.is_private {
            write!(f, " (private)")?;
        }
        writeln!(f, " by {}, {}", self.creator, self.created)?;
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

impl Render for Bug {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        output_field_wrapped!(f, "Summary", &self.summary, width);
        output_field!(f, "Assignee", &self.assigned_to, width);
        output_field!(f, "Creator", &self.creator, width);
        output_field!(f, "Created", &self.created, width);
        output_field!(f, "Updated", &self.updated, width);
        output_field!(f, "Deadline", &self.deadline, width);
        output_field!(f, "Status", &self.status, width);
        output_field!(f, "Resolution", &self.resolution, width);
        output_field!(f, "Duplicate of", &self.duplicate_of, width);
        output_field!(f, "Whiteboard", &self.whiteboard, width);
        output_field!(f, "Component", &self.component, width);
        output_field!(f, "Version", &self.version, width);
        output_field!(f, "Target", &self.target, width);
        output_field!(f, "Product", &self.product, width);
        output_field!(f, "Platform", &self.platform, width);
        output_field!(f, "OS", &self.op_sys, width);
        output_field!(f, "Priority", &self.priority, width);
        output_field!(f, "Severity", &self.severity, width);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;
        output_field!(f, "Alias", &self.alias, width);
        wrapped_csv(f, "Groups", &self.groups, width)?;
        wrapped_csv(f, "Keywords", &self.keywords, width)?;
        wrapped_csv(f, "CC", &self.cc, width)?;
        wrapped_csv(f, "Blocks", &self.blocks, width)?;
        wrapped_csv(f, "Depends on", &self.depends_on, width)?;
        output_field!(f, "URL", &self.url, width);
        if !self.see_also.is_empty() {
            truncated_list(f, "See also", &self.see_also, width)?;
        }

        if !self.comments.is_empty() {
            writeln!(f, "{:<12} : {}", "Comments", self.comments.len())?;
        }

        if !self.history.is_empty() {
            writeln!(f, "{:<12} : {}", "Changes", self.history.len())?;
        }

        if !self.attachments.is_empty() {
            writeln!(f, "\n{:<12} : {}", "Attachments", self.attachments.len())?;
            writeln!(f, "{}", "-".repeat(width))?;
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
