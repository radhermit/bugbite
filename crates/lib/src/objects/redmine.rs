use std::hash::{Hash, Hasher};

use chrono::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use std::fmt;

use crate::serde::null_empty_str;
use crate::service::redmine::IssueField;
use crate::traits::RenderSearch;

use super::{stringify, Item};

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(default)]
pub struct Issue {
    pub id: u64,
    pub assigned_to: Option<Person>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub status: Option<FieldValue>,
    pub tracker: Option<FieldValue>,
    pub priority: Option<FieldValue>,
    pub author: Option<Person>,
    #[serde(rename = "closed_on")]
    pub closed: Option<DateTime<Utc>>,
    #[serde(rename = "created_on")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "updated_on")]
    pub updated: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub comments: Vec<Comment>,
}

impl PartialEq for Issue {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Issue {}

impl Hash for Issue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
pub struct FieldValue {
    id: u64,
    name: String,
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Issue {
    pub fn search_display(&self) -> String {
        self.id.to_string()
    }
}

impl From<Issue> for Item {
    fn from(value: Issue) -> Self {
        Item::Redmine(Box::new(value))
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
pub struct Person {
    id: u64,
    name: String,
}

impl fmt::Display for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct Comment {
    /// The number of the comment local to the issue.
    ///
    /// The description is 0, comments start at 1.
    #[serde(default)]
    pub count: u64,
    #[serde(rename = "notes", default, deserialize_with = "null_empty_str")]
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
