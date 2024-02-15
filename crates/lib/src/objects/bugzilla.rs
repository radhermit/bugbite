use std::fmt;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::{Base64, Item};

#[derive(Deserialize, Serialize, Debug)]
pub struct Attachment {
    pub id: u64,
    pub bug_id: u64,
    pub file_name: String,
    pub summary: String,
    pub size: u64,
    pub creator: String,
    pub content_type: String,
    #[serde(rename = "creation_time")]
    pub created: DateTime<Utc>,
    #[serde(rename = "last_change_time")]
    pub updated: DateTime<Utc>,
    #[serde(default)]
    data: Base64,
}

impl Attachment {
    pub fn data(&self) -> &[u8] {
        &self.data.0
    }

    pub fn read(&self) -> std::borrow::Cow<str> {
        // TODO: auto-decompress standard archive formats
        String::from_utf8_lossy(&self.data.0)
    }
}

impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Attachment: [{}] [{}] ({}, {}) by {}, {}",
            self.id,
            self.file_name,
            format_size(self.size, BINARY),
            self.content_type,
            self.creator,
            self.updated
        )?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Comment {
    id: u64,
    count: u64,
    text: String,
    creator: String,
    #[serde(rename = "creation_time")]
    created: DateTime<Utc>,
}

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.count != 0 {
            write!(f, "Comment #{} ", self.count)?;
        } else {
            write!(f, "Description ")?;
        }
        writeln!(f, "by {}, {}", self.creator, self.created)?;
        writeln!(f, "{}", "-".repeat(80))?;
        writeln!(f, "{}", self.text.trim())?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(default)]
pub struct Bug {
    id: u64,
    assigned_to: Option<String>,
    #[serde(rename = "creator")]
    reporter: Option<String>,
    #[serde(rename = "creation_time")]
    created: Option<DateTime<Utc>>,
    #[serde(rename = "last_change_time")]
    updated: Option<DateTime<Utc>>,
    #[serde(rename = "alias")]
    aliases: Vec<String>,
    summary: Option<String>,
    status: Option<String>,
    whiteboard: Option<String>,
    cc: Vec<String>,
    blocks: Vec<u64>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) attachments: Vec<Attachment>,
}

impl From<Bug> for Item {
    fn from(value: Bug) -> Self {
        Item::Bugzilla(Box::new(value))
    }
}

impl fmt::Display for Bug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        if !self.comments.is_empty() {
            writeln!(f, "Comments: {}", self.comments.len())?;
        }
        if !self.attachments.is_empty() {
            writeln!(f, "Attachment: {}", self.attachments.len())?;
        }

        for attachment in &self.attachments {
            write!(f, "\n{attachment}")?;
        }
        for comment in &self.comments {
            write!(f, "\n{comment}")?;
        }

        Ok(())
    }
}

impl Bug {
    pub fn search_display(&self) -> String {
        let id = self.id;
        match (self.assigned_to.as_deref(), self.summary.as_deref()) {
            (Some(assignee), Some(summary)) => format!("{id:<8} {assignee:<20} {summary}"),
            (Some(assignee), None) => format!("{id:<8} {assignee}"),
            (None, Some(summary)) => format!("{id:<8} {summary}"),
            (None, None) => format!("{id}"),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn reporter(&self) -> Option<&str> {
        self.reporter.as_deref()
    }
}
