use chrono::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use std::fmt;

use crate::service::redmine::IssueField;
use crate::traits::RenderSearch;

use super::{stringify, Item};

#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct Issue {
    pub id: u64,
    pub assigned_to: Option<Person>,
    #[serde(rename = "subject")]
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "author")]
    pub creator: Option<Person>,
    #[serde(rename = "created_on")]
    pub created: Option<DateTime<Utc>>,
    #[serde(rename = "updated_on")]
    pub updated: Option<DateTime<Utc>>,
    pub comments: Vec<Comment>,
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
    pub count: u64,
    pub text: String,
    pub creator: Person,
    pub created: DateTime<Utc>,
}

impl RenderSearch<IssueField> for Issue {
    fn render(&self, fields: &[IssueField]) -> String {
        let field_to_string = |field: &IssueField| -> String {
            match field {
                IssueField::Id => format!("{:<8}", self.id),
                IssueField::AssignedTo => format!("{:<20}", stringify!(self.assigned_to)),
                IssueField::Summary => stringify!(self.summary),
                IssueField::Creator => format!("{:<20}", stringify!(self.creator)),
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