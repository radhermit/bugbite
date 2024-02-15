use std::fmt;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use super::Item;

#[derive(Deserialize, Serialize, Debug)]
pub struct Attachment {
    name: String,
}

impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Attachment: {}", self.name)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Comment {
    text: String,
}

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.text)?;
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
    #[serde(rename = "alias")]
    aliases: Vec<String>,
    summary: Option<String>,
    status: Option<String>,
    cc: Vec<String>,
    blocks: Vec<u64>,
    comments: Vec<Comment>,
    attachments: Vec<Attachment>,
}

impl From<Bug> for Item {
    fn from(value: Bug) -> Self {
        Item::Bugzilla(value)
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
        if let Some(data) = self.status.as_deref() {
            writeln!(f, "Status: {data}")?;
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
            write!(f, "{attachment}")?;
        }
        for comment in &self.comments {
            write!(f, "{comment}")?;
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
