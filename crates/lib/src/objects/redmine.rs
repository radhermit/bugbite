use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use chrono::prelude::*;
use indexmap::IndexSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, DefaultOnNull};

use std::fmt;

use crate::service::redmine::IssueField;
use crate::traits::RenderSearch;

use super::{stringify, Item};

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Issue {
    pub id: u64,
    pub assigned_to: Option<Person>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub status: Option<Field>,
    pub tracker: Option<Field>,
    pub priority: Option<Field>,
    pub author: Option<Person>,
    pub custom_fields: Option<IndexSet<CustomField>>,
    #[serde(rename = "closed_on")]
    pub closed: Option<DateTime<Utc>>,
    #[serde(rename = "created_on")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "updated_on")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub comments: Vec<Comment>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Field {
    id: u64,
    name: String,
}

impl Deref for Field {
    type Target = str;

    fn deref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CustomField {
    id: u64,
    pub name: String,
    pub value: CustomFieldValue,
}

impl PartialEq for CustomField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CustomField {}

impl Hash for CustomField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Borrow<str> for CustomField {
    fn borrow(&self) -> &str {
        &self.name
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum CustomFieldValue {
    String(String),
    Array(Vec<String>),
}

impl From<Issue> for Item {
    fn from(value: Issue) -> Self {
        Item::Redmine(Box::new(value))
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Person {
    id: u64,
    name: String,
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Comment {
    /// The number of the comment local to the issue.
    ///
    /// The description is 0, comments start at 1.
    #[serde(default)]
    pub count: u64,
    #[serde_as(deserialize_as = "DefaultOnNull")]
    #[serde(default, rename = "notes")]
    pub text: String,
    pub user: Person,
    #[serde(rename = "created_on")]
    pub created: DateTime<Utc>,
}

impl RenderSearch<IssueField> for Issue {
    fn render(&self, fields: &[IssueField]) -> String {
        let field_to_string = |field: &IssueField| -> String {
            match field {
                IssueField::Id => format!("{:<8}", self.id),
                IssueField::Assignee => format!("{:<20}", stringify!(self.assigned_to)),
                IssueField::Subject => stringify!(self.subject),
                IssueField::Status => format!("{:<20}", stringify!(self.status)),
                IssueField::Tracker => format!("{:<20}", stringify!(self.tracker)),
                IssueField::Priority => format!("{:<20}", stringify!(self.priority)),
                IssueField::Author => format!("{:<20}", stringify!(self.author)),
                IssueField::Closed => stringify!(self.closed),
                IssueField::Created => stringify!(self.created),
                IssueField::Updated => stringify!(self.updated),
            }
        };

        match fields {
            [] => panic!("no fields defined"),
            [field] => field_to_string(field).trim().to_string(),
            fields => fields.iter().map(field_to_string).join(" "),
        }
    }
}
