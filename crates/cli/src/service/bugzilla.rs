use std::process::ExitCode;

use bugbite::objects::bugzilla::*;
use bugbite::service::{
    bugzilla::{Config, Service},
    ClientBuilder,
};
use itertools::Itertools;
use tracing::info;

use crate::utils::truncate;

use super::output::*;
use super::Render;

mod attachment;
mod comment;
mod create;
mod get;
mod history;
mod search;
mod update;

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
    /// Attachment commands
    #[command(alias = "a")]
    Attachment(Box<attachment::Command>),

    /// Get bug comments
    Comment(comment::Command),

    /// Create bug
    #[command(alias = "c")]
    Create(Box<create::Command>),

    /// Get bugs
    #[command(alias = "g")]
    Get(get::Command),

    /// Get bug changes
    History(history::Command),

    /// Search bugs
    #[command(alias = "s")]
    Search(Box<search::Command>),

    /// Update bugs
    #[command(alias = "u")]
    Update(Box<update::Command>),
}

impl Subcommand {
    async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        match self {
            Self::Attachment(cmd) => cmd.run(service).await,
            Self::Comment(cmd) => cmd.run(service).await,
            Self::Create(cmd) => cmd.run(service).await,
            Self::Get(cmd) => cmd.run(service).await,
            Self::History(cmd) => cmd.run(service).await,
            Self::Update(cmd) => cmd.run(service).await,
            Self::Search(cmd) => cmd.run(service).await,
        }
    }
}

impl Render for Attachment {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        let deleted = if self.size == 0 { " (deleted)" } else { "" };
        let line = if self.summary != self.file_name {
            format!(
                "{}: {} ({}){deleted}",
                self.id, self.summary, self.file_name
            )
        } else {
            format!("{}: {}{deleted}", self.id, self.summary)
        };
        writeln!(f, "{}", truncate(&line, width))?;

        // output additional attachment info on request
        let line = format!(
            "  ({}) {}, created by {}, {}",
            if self.is_patch {
                "patch"
            } else {
                &self.content_type
            },
            self.human_size(),
            self.creator,
            self.updated
        );
        info!("{line}");

        Ok(())
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

impl Render for BugUpdate<'_> {
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
        output_field!(f, "QA", &self.qa_contact, width);
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
        wrapped_csv(f, "Alias", &self.alias, width)?;
        wrapped_csv(f, "Groups", &self.groups, width)?;
        wrapped_csv(f, "Keywords", &self.keywords, width)?;
        wrapped_csv(f, "CC", &self.cc, width)?;
        wrapped_csv(f, "Flags", &self.flags, width)?;
        wrapped_csv(f, "Tags", &self.tags, width)?;
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

        // render updates in order of occurrence
        for update in self.updates() {
            writeln!(f)?;
            update.render(f, width)?;
        }

        Ok(())
    }
}
