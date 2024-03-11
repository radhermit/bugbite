use std::cmp::Ordering;
use std::collections::HashSet;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize};

use crate::serde::{non_empty_str, null_empty_vec};
use crate::service::bugzilla::BugField;
use crate::traits::RenderSearch;

use super::{stringify, Base64, Item};

/// Common default values used for unset fields.
pub(crate) static UNSET_VALUES: Lazy<HashSet<String>> = Lazy::new(|| {
    ["unspecified", "Unspecified", "---", "--", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect()
});

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
enum Alias {
    List(Vec<String>),
    String(String),
    None,
}

/// Deserialize an alias field to a string.
fn alias_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Alias::deserialize(d).map(|o| match o {
        Alias::String(value) => {
            if !value.is_empty() {
                Some(value)
            } else {
                None
            }
        }
        Alias::List(values) => {
            if let Some(value) = values.into_iter().next() {
                if !value.is_empty() {
                    return Some(value);
                }
            }
            None
        }
        Alias::None => None,
    })
}

/// Deserialize a string field value setting common unset values to None.
pub(crate) fn unset_value_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    non_empty_str(d).map(|o| o.filter(|s| !UNSET_VALUES.contains(s)))
}

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct Bug {
    pub id: u64,
    #[serde(deserialize_with = "alias_str")]
    pub alias: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub assigned_to: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub creator: Option<String>,
    #[serde(rename = "creation_time")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "last_change_time")]
    pub updated: Option<DateTime<Utc>>,
    pub deadline: Option<NaiveDate>,
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
    #[serde(deserialize_with = "unset_value_str")]
    pub version: Option<String>,
    #[serde(deserialize_with = "unset_value_str")]
    pub platform: Option<String>,
    #[serde(deserialize_with = "unset_value_str")]
    pub op_sys: Option<String>,
    #[serde(rename = "target_milestone", deserialize_with = "unset_value_str")]
    pub target: Option<String>,
    #[serde(deserialize_with = "unset_value_str")]
    pub priority: Option<String>,
    #[serde(deserialize_with = "unset_value_str")]
    pub severity: Option<String>,
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
    #[serde(deserialize_with = "null_empty_vec")]
    pub see_also: Vec<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub url: Option<String>,
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
                BugField::Alias => format!("{:<20}", stringify!(self.alias)),
                BugField::AssignedTo => format!("{:<20}", stringify!(self.assigned_to)),
                BugField::Blocks => format!("{:<20}", self.blocks.iter().join(",")),
                BugField::Cc => format!("{:<20}", self.cc.iter().join(",")),
                BugField::Component => format!("{:<20}", stringify!(self.component)),
                BugField::Created => stringify!(self.created),
                BugField::Creator => format!("{:<20}", stringify!(self.creator)),
                BugField::Deadline => stringify!(self.deadline),
                BugField::DependsOn => format!("{:<20}", self.depends_on.iter().join(",")),
                BugField::Id => format!("{:<9}", self.id),
                BugField::Keywords => format!("{:<20}", self.keywords.iter().join(",")),
                BugField::Os => format!("{:<20}", stringify!(self.op_sys)),
                BugField::Platform => format!("{:<20}", stringify!(self.platform)),
                BugField::Priority => format!("{:<12}", stringify!(self.priority)),
                BugField::Product => format!("{:<20}", stringify!(self.product)),
                BugField::Resolution => format!("{:<20}", stringify!(self.resolution)),
                BugField::Severity => format!("{:<12}", stringify!(self.severity)),
                BugField::Status => format!("{:<20}", stringify!(self.status)),
                BugField::Summary => stringify!(self.summary),
                BugField::Target => format!("{:<20}", stringify!(self.target)),
                BugField::Updated => stringify!(self.updated),
                BugField::Url => format!("{:<20}", stringify!(self.url)),
                BugField::Version => format!("{:<20}", stringify!(self.version)),
                BugField::Whiteboard => format!("{:<20}", stringify!(self.whiteboard)),
            }
        };

        match fields {
            [] => panic!("no fields defined"),
            [field] => field_to_string(field).trim().to_string(),
            fields => fields.iter().map(field_to_string).join(" "),
        }
    }
}
