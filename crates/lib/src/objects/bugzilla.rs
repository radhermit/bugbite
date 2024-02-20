use std::cmp::Ordering;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use serde::{Deserialize, Serialize};

use crate::serde::{non_empty_str, null_empty_vec};

use super::{Base64, Item};

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
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

    pub fn human_size(&self) -> String {
        format_size(self.size, BINARY)
    }

    pub fn read(&self) -> std::borrow::Cow<str> {
        // TODO: auto-decompress standard archive formats
        String::from_utf8_lossy(&self.data.0)
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

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Comment {
    /// The number of the comment local to the bug.
    ///
    /// The description is 0, comments start at 1.
    pub id: u64,
    pub bug_id: u64,
    pub attachment_id: Option<u64>,
    pub count: u64,
    pub text: String,
    pub creator: String,
    #[serde(rename = "creation_time")]
    pub created: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Event {
    pub who: String,
    pub when: DateTime<Utc>,
    pub changes: Vec<Change>,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Change {
    pub field_name: String,
    #[serde(deserialize_with = "non_empty_str")]
    pub removed: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub added: Option<String>,
    pub attachment_id: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct Bug {
    pub id: u64,
    #[serde(deserialize_with = "non_empty_str")]
    pub assigned_to: Option<String>,
    #[serde(rename = "creator", deserialize_with = "non_empty_str")]
    pub reporter: Option<String>,
    #[serde(rename = "creation_time")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "last_change_time")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(rename = "alias", deserialize_with = "null_empty_vec")]
    pub aliases: Vec<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub summary: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub status: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub whiteboard: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub product: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub component: Option<String>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub cc: Vec<String>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub blocks: Vec<u64>,
    #[serde(rename = "depends_on", deserialize_with = "null_empty_vec")]
    pub depends: Vec<u64>,
    #[serde(rename = "see_also", deserialize_with = "null_empty_vec")]
    pub urls: Vec<String>,
    pub comments: Vec<Comment>,
    pub attachments: Vec<Attachment>,
    pub history: Vec<Event>,
}

impl From<Bug> for Item {
    fn from(value: Bug) -> Self {
        Item::Bugzilla(Box::new(value))
    }
}

impl Bug {
    pub fn events(&self) -> impl Iterator<Item = Modification> {
        let comments = self.comments.iter().map(Modification::Comment);
        let history = self.history.iter().map(Modification::Event);
        let mut events: Vec<_> = comments.chain(history).collect();
        events.sort();
        events.into_iter()
    }
}
