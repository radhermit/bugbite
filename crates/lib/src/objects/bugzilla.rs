use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use chrono::prelude::*;
use humansize::{format_size, BINARY};
use indexmap::IndexSet;
use itertools::{Either, Itertools};
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{
    serde_as, skip_serializing_none, BoolFromInt, DeserializeFromStr, SerializeDisplay,
};
use strum::{Display, EnumString};

use crate::serde::{non_empty_str, null_empty_set, null_empty_str, null_empty_vec};
use crate::service::bugzilla::{BugField, FilterField, GroupField};
use crate::traits::RenderSearch;
use crate::types::OrderedSet;
use crate::Error;

use super::{stringify, Base64, Item};

/// Common default values used for unset fields.
static UNSET_VALUES: Lazy<HashSet<String>> = Lazy::new(|| {
    ["unspecified", "Unspecified", "---", "--", "-"]
        .iter()
        .map(|s| s.to_string())
        .collect()
});

/// A file attachment on a bug.
#[serde_as]
#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Attachment {
    /// Unique attachment identifier.
    pub id: u64,

    /// Bug identifier the attachment is related to.
    pub bug_id: u64,

    /// File name of the attachment.
    pub file_name: String,

    /// Description of the attachment.
    pub summary: String,

    /// Size of the attachment in bytes.
    pub size: u64,

    /// Login identifier of the attachment's creator.
    pub creator: String,

    /// MIME type of the attachment.
    pub content_type: String,

    /// Attachment is private.
    #[serde_as(as = "BoolFromInt")]
    pub is_private: bool,

    /// Attachment is obsolete.
    #[serde_as(as = "BoolFromInt")]
    pub is_obsolete: bool,

    /// Attachment is a patch.
    #[serde_as(as = "BoolFromInt")]
    pub is_patch: bool,

    /// Creation time of the attachment.
    #[serde(rename = "creation_time")]
    pub created: DateTime<Utc>,

    /// Last update time of the attachment.
    #[serde(rename = "last_change_time")]
    pub updated: DateTime<Utc>,

    /// Flags of the attachment.
    #[serde(deserialize_with = "null_empty_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<BugFlag>,

    /// Attachment data.
    #[serde(default)]
    data: Base64,
}

impl AsRef<[u8]> for Attachment {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

// TODO: support auto-decompressing standard archive formats
impl Attachment {
    pub fn human_size(&self) -> String {
        format_size(self.size, BINARY)
    }

    /// Return true if the attachment has no data, otherwise false.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Return true if the attachment has been deleted, otherwise false.
    pub fn is_deleted(&self) -> bool {
        self.size == 0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BugUpdate<'a> {
    Comment(&'a Comment),
    Event(&'a Event),
}

impl BugUpdate<'_> {
    fn date(&self) -> &DateTime<Utc> {
        match self {
            Self::Comment(comment) => &comment.created,
            Self::Event(event) => &event.when,
        }
    }
}

impl Ord for BugUpdate<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date().cmp(other.date())
    }
}

impl PartialOrd for BugUpdate<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct Comment {
    /// Globally unique ID for the comment.
    pub id: u64,

    /// Bug ID the comment is on.
    pub bug_id: u64,

    /// Attachment ID related to the comment.
    pub attachment_id: Option<u64>,

    /// The number of the comment local to the bug.
    ///
    /// The description is 0, comments start at 1.
    pub count: usize,

    pub text: String,
    pub creator: String,
    #[serde(rename = "creation_time")]
    pub created: DateTime<Utc>,
    pub is_private: bool,
    #[serde(default)]
    pub tags: OrderedSet<String>,
}

impl Comment {
    /// Format a comment into a reply string.
    pub fn reply(&self) -> String {
        // TODO: pull real name for creator?
        let creator = self
            .creator
            .split_once('@')
            .map(|x| x.0)
            .unwrap_or(&self.creator);
        let mut data = vec![format!(
            "(In reply to {} from comment #{})",
            creator, self.count
        )];
        for line in self.text.lines() {
            data.push(format!("> {line}"));
        }
        data.iter().join("\n")
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct Event {
    pub who: String,
    pub when: DateTime<Utc>,
    pub changes: Vec<Change>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct Change {
    pub field_name: String,
    #[serde(deserialize_with = "non_empty_str")]
    pub removed: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub added: Option<String>,
    pub attachment_id: Option<u64>,
}

#[derive(
    Display,
    EnumString,
    DeserializeFromStr,
    SerializeDisplay,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
)]
pub enum FlagStatus {
    #[strum(serialize = "+")]
    Granted,
    #[strum(serialize = "-")]
    Denied,
    #[strum(serialize = "?")]
    Requested,
    #[strum(serialize = "X")]
    Remove,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct Flag {
    pub name: String,
    pub status: FlagStatus,
}

impl FromStr for Flag {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if s.is_empty() {
            return Err(Error::InvalidValue("empty flag".to_string()));
        }

        let name = &s[..s.len() - 1];
        let status = &s[s.len() - 1..];
        let status = status
            .parse()
            .map_err(|_| Error::InvalidValue(format!("invalid flag status: {status}")))?;

        Ok(Self {
            name: name.to_string(),
            status,
        })
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.name, self.status)
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct BugFlag {
    #[serde(flatten)]
    pub flag: Flag,
    pub setter: String,
    #[serde(rename = "creation_date")]
    pub created: DateTime<Utc>,
    #[serde(rename = "modification_date")]
    pub updated: DateTime<Utc>,
}

impl fmt::Display for BugFlag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.flag)
    }
}

// Support deserializing alias field from string or array, the current Bugzilla 5.0
// webservice API returns alias arrays while Mozilla upstream uses string values which
// is what Bugzilla is moving to in the future (see
// https://bugzilla.mozilla.org/show_bug.cgi?id=1534305).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
enum Alias {
    List(OrderedSet<String>),
    String(String),
}

/// Deserialize a string field value setting common unset values to None.
pub(crate) fn unset_value_str<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    non_empty_str(d).map(|o| o.filter(|s| !UNSET_VALUES.contains(s)))
}

/// Deserialize an alias as an ordered set of strings.
pub(crate) fn alias_to_set<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<OrderedSet<String>, D::Error> {
    Option::<Alias>::deserialize(d).map(|o| match o {
        Some(Alias::List(values)) => values,
        Some(Alias::String(value)) => [value].into_iter().collect(),
        None => Default::default(),
    })
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct Bug {
    pub id: u64,
    #[serde(deserialize_with = "alias_to_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub alias: OrderedSet<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub assigned_to: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub qa_contact: Option<String>,
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
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub groups: OrderedSet<String>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub keywords: OrderedSet<String>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub cc: OrderedSet<String>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub blocks: OrderedSet<u64>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub depends_on: OrderedSet<u64>,
    #[serde(rename = "dupe_of")]
    pub duplicate_of: Option<u64>,
    #[serde(deserialize_with = "null_empty_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<BugFlag>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub tags: OrderedSet<String>,
    #[serde(deserialize_with = "null_empty_set")]
    #[serde(skip_serializing_if = "OrderedSet::is_empty")]
    pub see_also: OrderedSet<String>,
    #[serde(deserialize_with = "non_empty_str")]
    pub url: Option<String>,
    #[serde(skip)]
    pub comments: Vec<Comment>,
    #[serde(skip)]
    pub attachments: Vec<Attachment>,
    #[serde(skip)]
    pub history: Vec<Event>,
}

impl From<Bug> for Item {
    fn from(value: Bug) -> Self {
        Item::Bugzilla(Box::new(value))
    }
}

impl Bug {
    pub fn updates(&self) -> impl Iterator<Item = BugUpdate> {
        let comments = self.comments.iter().map(BugUpdate::Comment);
        let history = self.history.iter().map(BugUpdate::Event);
        comments.chain(history).sorted()
    }
}

impl RenderSearch<BugField> for Bug {
    fn render(&self, fields: &[BugField]) -> String {
        let field_to_string = |field: &BugField| -> String {
            match field {
                BugField::Alias => format!("{:<20}", self.alias.iter().join(",")),
                BugField::Assignee => format!("{:<20}", stringify!(self.assigned_to)),
                BugField::Blocks => format!("{:<20}", self.blocks.iter().join(",")),
                BugField::Cc => format!("{:<20}", self.cc.iter().join(",")),
                BugField::Component => format!("{:<20}", stringify!(self.component)),
                BugField::Created => stringify!(self.created),
                BugField::Creator => format!("{:<20}", stringify!(self.creator)),
                BugField::Deadline => stringify!(self.deadline),
                BugField::Depends => format!("{:<20}", self.depends_on.iter().join(",")),
                BugField::DuplicateOf => format!("{:<9}", stringify!(self.duplicate_of)),
                BugField::Flags => format!("{:<20}", self.flags.iter().join(",")),
                BugField::Id => format!("{:<9}", self.id),
                BugField::Keywords => format!("{:<20}", self.keywords.iter().join(",")),
                BugField::Os => format!("{:<20}", stringify!(self.op_sys)),
                BugField::Platform => format!("{:<20}", stringify!(self.platform)),
                BugField::Priority => format!("{:<12}", stringify!(self.priority)),
                BugField::Product => format!("{:<20}", stringify!(self.product)),
                BugField::Qa => format!("{:<20}", stringify!(self.qa_contact)),
                BugField::Resolution => format!("{:<20}", stringify!(self.resolution)),
                BugField::SeeAlso => format!("{:<20}", self.see_also.iter().join(",")),
                BugField::Severity => format!("{:<12}", stringify!(self.severity)),
                BugField::Status => format!("{:<20}", stringify!(self.status)),
                BugField::Summary => stringify!(self.summary),
                BugField::Tags => format!("{:<20}", self.tags.iter().join(",")),
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

impl RenderSearch<FilterField> for Bug {
    fn render(&self, fields: &[FilterField]) -> String {
        let (bug_fields, group_fields): (Vec<_>, Vec<_>) =
            fields.iter().partition_map(|x| match x {
                FilterField::Bug(x) => Either::Left(*x),
                FilterField::Group(x) => Either::Right(*x),
            });
        match (&bug_fields[..], &group_fields[..]) {
            (fields @ [_, ..], _) => self.render(fields),
            ([], x) if x.contains(&GroupField::All) || x.contains(&GroupField::Default) => {
                self.render(&[BugField::Id, BugField::Summary])
            }
            _ => self.render(&[BugField::Id]),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BugzillaField {
    id: u64,
    #[serde(rename = "type")]
    kind: u64,
    is_custom: bool,
    name: String,
    display_name: String,
    is_mandatory: bool,
    #[serde(default)]
    values: Vec<BugzillaFieldValue>,
}

impl fmt::Display for BugzillaField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Name: {}", self.name)?;
        write!(f, "Display name: {}", self.display_name)?;
        if !self.values.is_empty() {
            let values = self
                .values
                .iter()
                .filter(|x| !x.name.is_empty())
                .map(|x| x.name.as_str())
                .collect::<IndexSet<_>>();
            let values = values.iter().join(", ");
            write!(f, "\nValues: {values}")?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BugzillaFieldValue {
    #[serde(deserialize_with = "null_empty_str")]
    name: String,
    sort_key: Option<u64>,
    description: Option<String>,
    is_open: Option<bool>,
}
