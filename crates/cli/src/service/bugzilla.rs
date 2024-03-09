use std::process::ExitCode;

use bugbite::client::{bugzilla::Client, ClientBuilder};
use bugbite::objects::bugzilla::*;
use bugbite::service::bugzilla::Config;
use itertools::Itertools;

use crate::utils::truncate;

use super::output::*;
use super::{auth_required, auth_retry, Render};

mod attach;
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
    #[arg(short, long, requires = "password", conflicts_with = "api_key")]
    user: Option<String>,

    /// Bugzilla password
    #[arg(short, long, requires = "user", conflicts_with = "api_key")]
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
        config.api_key = self.auth.api_key;
        config.user = self.auth.user;
        config.password = self.auth.password;

        let client = Client::new(config, builder.build())?;
        Ok(auth_retry(|| self.cmd.run(&client))?)
    }
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Attach files to bugs
    #[command(alias = "at")]
    Attach(attach::Command),
    /// Get attachments
    #[command(alias = "a")]
    Attachments(attachments::Command),
    /// Get comments
    Comments(comments::Command),
    /// Get bugs
    #[command(alias = "g")]
    Get(get::Command),
    /// Get bug history
    History(history::Command),
    /// Search bugs
    #[command(alias = "s")]
    Search(search::Command),
}

impl Subcommand {
    fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        match self {
            Self::Attach(cmd) => auth_required(|| cmd.run(client)),
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

impl Render for Bug {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()> {
        output_field_wrapped!(f, "Summary", &self.summary, width);
        output_field!(f, "Assignee", &self.assigned_to);
        output_field!(f, "Creator", &self.creator);
        output_field!(f, "Created", &self.created);
        output_field!(f, "Updated", &self.updated);
        output_field!(f, "Status", &self.status);
        output_field!(f, "Resolution", &self.resolution);
        output_field!(f, "Duplicate of", &self.duplicate_of);
        output_field!(f, "Whiteboard", &self.whiteboard);
        output_field!(f, "Product", &self.product);
        output_field!(f, "Component", &self.component);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;
        output_field!(f, "Alias", self.alias.as_ref().and_then(|x| x.display()));
        wrapped_csv(f, "CC", &self.cc, width)?;
        wrapped_csv(f, "Blocks", &self.blocks, width)?;
        wrapped_csv(f, "Depends on", &self.depends_on, width)?;
        if !self.urls.is_empty() {
            truncated_list(f, "See also", &self.urls, width)?;
        }

        // don't count the bug description as a comment
        if self.comments.len() > 1 {
            writeln!(f, "{:<12} : {}", "Comments", self.comments.len() - 1)?;
        }

        if !self.history.is_empty() {
            writeln!(f, "{:<12} : {}", "Changes", self.history.len())?;
        }

        if !self.attachments.is_empty() {
            writeln!(f, "{:<12} : {}\n", "Attachments", self.attachments.len())?;
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
