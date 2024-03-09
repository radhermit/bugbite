use std::cmp::Ordering;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::serde::{non_empty_str, null_empty_vec};
use crate::service::bugzilla::BugField;
use crate::traits::RenderSearch;

use super::{stringify, Base64, Item};

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

// Support deserializing alias field from string or array, the current Bugzilla 5.0
// webservice API returns alias arrays while Mozilla upstream uses string values which
// is what Bugzilla is moving to in the future (see
// https://bugzilla.mozilla.org/show_bug.cgi?id=1534305).
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Alias {
    List(Vec<String>),
    String(String),
}

impl Alias {
    /// Return the main, non-empty alias if it exists.
    pub fn display(&self) -> Option<&str> {
        match self {
            Self::String(value) if !value.is_empty() => Some(value.as_str()),
            Self::List(values) if !values.is_empty() && !values[0].is_empty() => {
                Some(values[0].as_str())
            }
            _ => None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct Bug {
    pub id: u64,
    pub alias: Option<Alias>,
    #[serde(deserialize_with = "non_empty_str")]
    pub assigned_to: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub creator: Option<String>,
    #[serde(rename = "creation_time")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "last_change_time")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "non_empty_str")]
    pub summary: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub status: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub resolution: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub whiteboard: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub product: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub component: Option<String>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub keywords: Vec<String>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub cc: Vec<String>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub blocks: Vec<u64>,
    #[serde(deserialize_with = "null_empty_vec")]
    pub depends_on: Vec<u64>,
    #[serde(rename = "dupe_of")]
    pub duplicate_of: Option<u64>,
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

impl RenderSearch<BugField> for Bug {
    fn render(&self, fields: &[BugField]) -> String {
        let field_to_string = |field: &BugField| -> String {
            match field {
                BugField::Id => format!("{:<8}", self.id),
                BugField::AssignedTo => format!("{:<20}", stringify!(self.assigned_to)),
                BugField::Summary => stringify!(self.summary),
                BugField::Creator => format!("{:<20}", stringify!(self.creator)),
                BugField::Created => stringify!(self.created),
                BugField::Updated => stringify!(self.updated),
                BugField::Status => format!("{:<20}", stringify!(self.status)),
                BugField::Resolution => format!("{:<20}", stringify!(self.resolution)),
                BugField::Whiteboard => format!("{:<20}", stringify!(self.whiteboard)),
                BugField::Product => format!("{:<20}", stringify!(self.product)),
                BugField::Component => format!("{:<20}", stringify!(self.component)),
                BugField::DependsOn => format!("{:<20}", self.depends_on.iter().join(",")),
                BugField::Keywords => format!("{:<20}", self.keywords.iter().join(",")),
            }
        };

        match fields {
            [] => panic!("no fields defined"),
            [field] => field_to_string(field).trim().to_string(),
            fields => fields.iter().map(field_to_string).join(" "),
        }
    }
}
