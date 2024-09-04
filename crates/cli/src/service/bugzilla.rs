use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::objects::bugzilla::*;
use bugbite::service::bugzilla::{self, Service};
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

#[derive(Args)]
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

#[derive(Args)]
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
        config.auth.merge(self.auth);
        config.client.merge(self.service);

        let service = Service::from_config(config)?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand)]
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
    async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
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

impl Render<&Attachment> for Service {
    fn render<W>(&self, item: &Attachment, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        let obsolete = if item.is_obsolete { " (obsolete)" } else { "" };
        let deleted = if item.is_deleted() { " (deleted)" } else { "" };
        let line = if item.summary != item.file_name {
            format!(
                "{}: {} ({}){obsolete}{deleted}",
                item.id, item.summary, item.file_name
            )
        } else {
            format!("{}: {}{deleted}", item.id, item.summary)
        };

        // don't output obsolete or deleted attachments by default
        if (!item.is_obsolete && !item.is_deleted()) || OUTDATED.load(Ordering::Acquire) {
            writeln!(f, "{}", truncate(f, &line, width))?;
        }

        // output additional attachment info on request
        let line = format!(
            "  ({}) {}, created by {}, {}",
            if item.is_patch {
                "patch"
            } else {
                &item.content_type
            },
            item.human_size(),
            item.creator,
            item.updated
        );
        verbose!(f, "{line}")?;

        Ok(())
    }
}

impl Render<&Comment> for Service {
    fn render<W>(&self, item: &Comment, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        if item.count != 0 {
            write!(f, "Comment #{}", item.count)?;
        } else {
            write!(f, "Description")?;
        }
        if !item.tags.is_empty() {
            write!(f, " ({})", item.tags.iter().join(", "))?;
        }
        if item.is_private {
            write!(f, " (private)")?;
        }
        writeln!(f, " by {}, {}", item.creator, item.created)?;
        writeln!(f, "{}", "-".repeat(width))?;
        // wrap comment text
        let wrapped = textwrap::wrap(item.text.trim(), width);
        writeln!(f, "{}", wrapped.iter().join("\n"))
    }
}

impl Render<&Event> for Service {
    fn render<W>(&self, item: &Event, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        if !item.changes.is_empty() {
            writeln!(f, "Changes made by {}, {}", item.who, item.when)?;
            writeln!(f, "{}", "-".repeat(width))?;
            for change in &item.changes {
                self.render(change, f, width)?;
            }
        }
        Ok(())
    }
}

impl Render<&Change> for Service {
    fn render<W>(&self, item: &Change, f: &mut W, _width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        let name = &item.field_name;
        match (item.removed.as_deref(), item.added.as_deref()) {
            (Some(removed), None) => writeln!(f, "{name}: -{removed}"),
            (Some(removed), Some(added)) => writeln!(f, "{name}: {removed} -> {added}"),
            (None, Some(added)) => writeln!(f, "{name}: +{added}"),
            (None, None) => panic!("invalid change"),
        }
    }
}

impl Render<&BugUpdate<'_>> for Service {
    fn render<W>(&self, item: &BugUpdate, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        match item {
            BugUpdate::Comment(comment) => self.render(*comment, f, width),
            BugUpdate::Event(event) => self.render(*event, f, width),
        }
    }
}

impl Render<&Bug> for Service {
    fn render<W>(&self, item: &Bug, f: &mut W, width: usize) -> io::Result<()>
    where
        W: IsTerminal + Write,
    {
        output_field_wrapped!(f, "Summary", &item.summary, width);
        output_field!(f, "Assignee", &item.assigned_to, width);
        output_field!(f, "QA", &item.qa_contact, width);
        output_field!(f, "Creator", &item.creator, width);
        output_field!(f, "Created", &item.created, width);
        output_field!(f, "Updated", &item.updated, width);
        output_field!(f, "Deadline", &item.deadline, width);
        output_field!(f, "Status", &item.status, width);
        output_field!(f, "Resolution", &item.resolution, width);
        output_field!(f, "Duplicate of", &item.duplicate_of, width);
        output_field!(f, "Whiteboard", &item.whiteboard, width);
        output_field!(f, "Component", &item.component, width);
        output_field!(f, "Version", &item.version, width);
        output_field!(f, "Target", &item.target, width);
        output_field!(f, "Product", &item.product, width);
        output_field!(f, "Platform", &item.platform, width);
        output_field!(f, "OS", &item.op_sys, width);
        output_field!(f, "Priority", &item.priority, width);
        output_field!(f, "Severity", &item.severity, width);
        writeln!(f, "{:<12} : {}", "ID", item.id)?;
        wrapped_csv(f, "Alias", &item.alias, width)?;
        wrapped_csv(f, "Groups", &item.groups, width)?;
        wrapped_csv(f, "Keywords", &item.keywords, width)?;
        wrapped_csv(f, "CC", &item.cc, width)?;
        wrapped_csv(f, "Flags", &item.flags, width)?;
        wrapped_csv(f, "Tags", &item.tags, width)?;
        wrapped_csv(f, "Blocks", &item.blocks, width)?;
        wrapped_csv(f, "Depends on", &item.depends_on, width)?;
        output_field!(f, "URL", &item.url, width);
        if !item.see_also.is_empty() {
            truncated_list(f, "See also", &item.see_also, width)?;
        }

        // TODO: handle different custom field value types
        for (name, value) in &item.custom_fields {
            let options = textwrap::Options::new(width - 15).subsequent_indent(&INDENT);
            let wrapped = textwrap::wrap(value, &options);
            let data = wrapped.iter().join("\n");
            writeln!(f, "{name:<12} : {data}")?;
        }

        if !item.comments.is_empty() {
            writeln!(f, "{:<12} : {}", "Comments", item.comments.len())?;
        }

        if !item.history.is_empty() {
            writeln!(f, "{:<12} : {}", "Changes", item.history.len())?;
        }

        if !item.attachments.is_empty() {
            writeln!(f, "\n{:<12} : {}", "Attachments", item.attachments.len())?;
            writeln!(f, "{}", "-".repeat(width))?;
            for attachment in &item.attachments {
                self.render(attachment, f, width)?;
            }
        }

        // render updates in order of occurrence
        for update in item.updates() {
            writeln!(f)?;
            self.render(&update, f, width)?;
        }

        Ok(())
    }
}
