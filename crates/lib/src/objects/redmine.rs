use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use chrono::prelude::*;
use indexmap::IndexSet;
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::{DefaultOnNull, serde_as, skip_serializing_none};

use std::fmt;

use crate::service::redmine::IssueField;
use crate::traits::RenderSearch;

use super::stringify;

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Issue {
    pub id: u64,
    pub assigned_to: Option<Person>,
    pub subject: Option<String>,
    pub status: Option<Field>,
    pub tracker: Option<Field>,
    pub priority: Option<Field>,
    pub author: Option<Person>,
    pub custom_fields: Option<IndexSet<CustomField>>,
    pub closed: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    pub comments: Vec<Comment>,
}

#[derive(Deserialize, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct IssueRaw {
    id: u64,
    assigned_to: Option<Person>,
    subject: Option<String>,
    status: Option<Field>,
    tracker: Option<Field>,
    priority: Option<Field>,
    author: Option<Person>,
    #[serde(default, deserialize_with = "skip_null_fields")]
    custom_fields: Option<IndexSet<CustomField>>,
    closed_on: Option<DateTime<Utc>>,
    created_on: Option<DateTime<Utc>>,
    updated_on: Option<DateTime<Utc>>,

    description: Option<String>,
    journals: Vec<Comment>,
}

impl From<IssueRaw> for Issue {
    fn from(mut value: IssueRaw) -> Self {
        let mut issue = Self {
            id: value.id,
            assigned_to: value.assigned_to,
            subject: value.subject,
            status: value.status,
            tracker: value.tracker,
            priority: value.priority,
            author: value.author,
            custom_fields: value.custom_fields,
            closed: value.closed_on,
            created: value.created_on,
            updated: value.updated_on,
            comments: Default::default(),
        };

        // treat description as a comment
        let mut count = 0;
        if let Some(text) = value.description.take() {
            issue.comments.push(Comment {
                count,
                text,
                user: issue.author.clone().unwrap(),
                created: issue.created.unwrap(),
            });
        }

        // TODO: handle parsing changes within journal data
        for mut comment in value.journals {
            if !comment.text.is_empty() {
                count += 1;
                comment.count = count;
                issue.comments.push(comment);
            }
        }

        issue
    }
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

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum CustomFieldValue {
    String(String),
    Array(Vec<String>),
    None,
}

/// Deserializing function for custom fields that skips null values.
fn skip_null_fields<'de, D>(deserializer: D) -> Result<Option<IndexSet<CustomField>>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut fields: Option<IndexSet<CustomField>> = Deserialize::deserialize(deserializer)?;

    if let Some(values) = fields.as_mut() {
        values.retain(|x| match &x.value {
            CustomFieldValue::None => false,
            CustomFieldValue::Array(values) if values.is_empty() => false,
            CustomFieldValue::String(value) if value.is_empty() => false,
            _ => true,
        });
    }

    match fields {
        Some(set) if !set.is_empty() => Ok(Some(set)),
        _ => Ok(None),
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
