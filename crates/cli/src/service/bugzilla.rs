use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::objects::bugzilla::*;
use bugbite::service::bugzilla::{self, Bugzilla};
use bugbite::service::ServiceKind;
use bugbite::traits::Merge;
use clap::Args;
use itertools::Itertools;
use tracing::debug;

use crate::utils::{truncate, verbose};

use super::output::*;
use super::Render;

mod attachment;
mod comment;
mod create;
mod fields;
mod get;
mod history;
mod search;
mod update;
mod version;

#[derive(Args, Debug)]
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

impl From<Authentication> for bugzilla::Authentication {
    fn from(value: Authentication) -> Self {
        Self {
            key: value.key,
            user: value.user,
            password: value.password,
        }
    }
}

#[derive(Args, Debug)]
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
            .get_kind(ServiceKind::Bugzilla, connection)?
            .into_bugzilla()
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
    /// Attachment commands
    #[command(alias = "a")]
    Attachment(Box<attachment::Command>),

    /// Get bug comments
    Comment(comment::Command),

    /// Create bug
    #[command(alias = "c")]
    Create(Box<create::Command>),

    /// Get bugzilla fields
    Fields(fields::Command),

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

    /// Get bugzilla version
    Version(version::Command),
}

impl Subcommand {
    async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Attachment(cmd) => cmd.run(service, f).await,
            Self::Comment(cmd) => cmd.run(service, f).await,
            Self::Create(cmd) => cmd.run(service, f).await,
            Self::Fields(cmd) => cmd.run(service, f).await,
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::History(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
            Self::Update(cmd) => cmd.run(service, f).await,
            Self::Version(cmd) => cmd.run(service, f).await,
        }
    }
}

static OUTDATED: AtomicBool = AtomicBool::new(false);

impl Render for Attachment {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        let obsolete = if self.is_obsolete { " (obsolete)" } else { "" };
        let deleted = if self.is_deleted() { " (deleted)" } else { "" };
        let line = if self.summary != self.file_name {
            format!(
                "{}: {} ({}){obsolete}{deleted}",
                self.id, self.summary, self.file_name
            )
        } else {
            format!("{}: {}{deleted}", self.id, self.summary)
        };

        // don't output obsolete or deleted attachments by default
        if (!self.is_obsolete && !self.is_deleted()) || OUTDATED.load(Ordering::Acquire) {
            writeln!(f, "{}", truncate(f, &line, width))?;
        }

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
        verbose!(f, "{line}")?;

        Ok(())
    }
}

impl Render for Comment {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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
    fn render<W>(&self, f: &mut W, _width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        match self {
            BugUpdate::Comment(comment) => comment.render(f, width),
            BugUpdate::Event(event) => event.render(f, width),
        }
    }
}

impl Render for Bug {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
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

        // TODO: handle different custom field value types
        for (name, value) in &self.custom_fields {
            let options = textwrap::Options::new(width - 15).subsequent_indent(&INDENT);
            let wrapped = textwrap::wrap(value, &options);
            let data = wrapped.iter().join("\n");
            writeln!(f, "{name:<12} : {data}")?;
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
