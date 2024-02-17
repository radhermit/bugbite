use std::cmp::Ordering;
use std::fmt;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::serde::non_empty_str;

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

#[derive(Debug, Eq, PartialEq)]
pub enum Modification<'a> {
    Comment(&'a Comment),
    Event(&'a Event),
}

impl Modification<'_> {
    fn date(&self) -> &DateTime<Utc> {
        match self {
            Self::Comment(comment) => &comment.created,
            Self::Event(event) => &event.when,
        }
    }
}

impl Ord for Modification<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date().cmp(other.date())
    }
}

impl PartialOrd for Modification<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Modification<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Comment(comment) => write!(f, "{comment}"),
            Self::Event(event) => write!(f, "{event}"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
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
        // TODO: pass in COLUMNS value?
        writeln!(f, "{}", "-".repeat(80))?;
        writeln!(f, "{}", self.text.trim())?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Event {
    who: String,
    when: DateTime<Utc>,
    changes: Vec<Change>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.changes.is_empty() {
            writeln!(f, "Changes made by {}, {}", self.who, self.when)?;
            // TODO: pass in COLUMNS value?
            writeln!(f, "{}", "-".repeat(80))?;
            for change in &self.changes {
                writeln!(f, "{change}")?;
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Change {
    field_name: String,
    #[serde(deserialize_with = "non_empty_str")]
    removed: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    added: Option<String>,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = &self.field_name;
        match (self.removed.as_deref(), self.added.as_deref()) {
            (Some(removed), None) => write!(f, "{name}: -{removed}"),
            (Some(removed), Some(added)) => write!(f, "{name}: {removed} -> {added}"),
            (None, Some(added)) => write!(f, "{name}: +{added}"),
            (None, None) => panic!("invalid change"),
        }
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
    #[serde(deserialize_with = "non_empty_str")]
    summary: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    status: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    whiteboard: Option<String>,
    cc: Vec<String>,
    blocks: Vec<u64>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) attachments: Vec<Attachment>,
    pub(crate) history: Vec<Event>,
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
            writeln!(f, "Attachment: {}\n", self.attachments.len())?;
            for attachment in &self.attachments {
                write!(f, "{attachment}")?;
            }
        }

        for e in self.events() {
            write!(f, "\n{e}")?;
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

    pub fn events(&self) -> impl Iterator<Item = Modification> {
        let comments = self.comments.iter().map(Modification::Comment);
        let history = self.history.iter().map(Modification::Event);
        let mut events: Vec<_> = comments.chain(history).collect();
        events.sort();
        events.into_iter()
    }
}
