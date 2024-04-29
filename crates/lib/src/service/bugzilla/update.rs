use std::hash::Hash;
use std::str::FromStr;
use std::{fmt, fs};

use camino::{Utf8Path, Utf8PathBuf};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::objects::{bugzilla::Flag, Range};
use crate::serde::non_empty_str;
use crate::traits::{Contains, InjectAuth, Request, WebService};
use crate::Error;

use super::comment::CommentRequest;

/// Changes made to a field.
#[derive(Deserialize, Debug, Eq, PartialEq, Hash)]
pub struct FieldChange {
    #[serde(deserialize_with = "non_empty_str")]
    added: Option<String>,
    #[serde(deserialize_with = "non_empty_str")]
    removed: Option<String>,
}

impl fmt::Display for FieldChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.removed.as_ref(), self.added.as_ref()) {
            (Some(removed), Some(added)) => write!(f, "{removed} -> {added}"),
            (Some(removed), None) => write!(f, "-{removed}"),
            (None, Some(added)) => write!(f, "+{added}"),
            (None, None) => panic!("invalid FieldChange"),
        }
    }
}

/// Changes made to a bug.
#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct BugChange {
    id: u64,
    comment: Option<Comment>,
    changes: IndexMap<String, FieldChange>,
}

impl fmt::Display for BugChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "=== Bug #{} ===", self.id)?;
        write!(f, "--- Updated fields ---")?;
        if !self.changes.is_empty() {
            for (name, change) in &self.changes {
                write!(f, "\n{name}: {change}")?;
            }
        } else {
            write!(f, "\nNone")?;
        }

        if let Some(comment) = self.comment.as_ref() {
            write!(f, "\n--- Added comment ---")?;
            write!(f, "\n{comment}")?;
        }

        Ok(())
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
pub enum RangeOrSet<T: FromStr + PartialOrd + Eq + Hash> {
    Range(Range<T>),
    Set(IndexSet<T>),
}

impl<T: FromStr + PartialOrd + Eq + Hash> FromStr for RangeOrSet<T>
where
    <T as FromStr>::Err: fmt::Display + fmt::Debug,
{
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse() {
            Ok(Self::Range(value))
        } else {
            let mut set = IndexSet::new();
            for x in s.split(',') {
                let value = x
                    .parse()
                    .map_err(|e| Error::InvalidValue(format!("invalid value: {e}")))?;
                set.insert(value);
            }
            Ok(Self::Set(set))
        }
    }
}

impl<T: fmt::Display + FromStr + PartialOrd + Eq + Hash> fmt::Display for RangeOrSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Range(value) => value.fmt(f),
            Self::Set(values) => write!(f, "{}", values.into_iter().join(",")),
        }
    }
}

impl<T: FromStr + PartialOrd + Eq + Hash> Contains<T> for RangeOrSet<T> {
    fn contains(&self, obj: &T) -> bool {
        match self {
            Self::Range(value) => value.contains(obj),
            Self::Set(value) => value.contains(obj),
        }
    }
}

#[derive(Debug)]
pub struct UpdateRequest {
    url: url::Url,
    ids: Vec<String>,
    params: Parameters,
}

impl Request for UpdateRequest {
    type Output = Vec<BugChange>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let params = self.params.encode(service, self.ids).await?;
        let request = service.client.put(self.url).json(&params).auth(service)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        let mut changes: Vec<BugChange> = serde_json::from_value(data)
            .map_err(|e| Error::InvalidValue(format!("failed deserializing changes: {e}")))?;
        if let Some(comment) = params.comment.as_ref() {
            for change in changes.iter_mut() {
                change.comment = Some(comment.clone());
            }
        }
        Ok(changes)
    }
}

impl UpdateRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        params: Parameters,
    ) -> crate::Result<Self>
    where
        S: fmt::Display,
    {
        let [id, ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        Ok(Self {
            url: service.config.base.join(&format!("rest/bug/{id}"))?,
            ids: ids.iter().map(|x| x.to_string()).collect(),
            params,
        })
    }
}

/// Supported change variants for set-based fields.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Eq, PartialEq, Clone)]
pub enum SetChange<T: Clone> {
    Add(T),
    Remove(T),
    Set(T),
}

impl<T: FromStr + Clone> FromStr for SetChange<T> {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Some(value) = s.strip_prefix('+') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Add(value))
        } else if let Some(value) = s.strip_prefix('-') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Remove(value))
        } else {
            let value = s
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Self::Set(value))
        }
    }
}

impl<T: FromStr + Clone + fmt::Display> fmt::Display for SetChange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Add(value) => write!(f, "+{value}"),
            Self::Remove(value) => write!(f, "-{value}"),
            Self::Set(value) => value.fmt(f),
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct SetChanges<T> {
    add: Option<Vec<T>>,
    remove: Option<Vec<T>>,
    set: Option<Vec<T>>,
}

impl<T: FromStr + Clone> FromIterator<SetChange<T>> for SetChanges<T> {
    fn from_iter<I: IntoIterator<Item = SetChange<T>>>(iterable: I) -> Self {
        let (mut add, mut remove, mut set) = (vec![], vec![], vec![]);
        for change in iterable {
            match change {
                SetChange::Add(value) => add.push(value),
                SetChange::Remove(value) => remove.push(value),
                SetChange::Set(value) => set.push(value),
            }
        }

        let set = if !set.is_empty() || (add.is_empty() && remove.is_empty()) {
            Some(set)
        } else {
            None
        };

        Self {
            add: Some(add),
            remove: Some(remove),
            set,
        }
    }
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Changes<T> {
    add: Option<Vec<T>>,
    remove: Option<Vec<T>>,
}

impl<T: FromStr + Clone> FromIterator<SetChange<T>> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = SetChange<T>>>(iterable: I) -> Self {
        let (mut add, mut remove) = (vec![], vec![]);
        for change in iterable {
            match change {
                SetChange::Add(value) | SetChange::Set(value) => add.push(value),
                SetChange::Remove(value) => remove.push(value),
            }
        }

        Self {
            add: Some(add),
            remove: Some(remove),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
struct Comment {
    body: String,
    is_private: bool,
}

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.body)
    }
}

/// Bug update parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Parameters {
    pub alias: Option<Vec<SetChange<String>>>,
    pub assignee: Option<String>,
    pub blocks: Option<Vec<SetChange<u64>>>,
    pub cc: Option<Vec<SetChange<String>>>,
    pub comment: Option<String>,
    pub comment_from: Option<Utf8PathBuf>,
    pub comment_is_private: Option<bool>,
    pub comment_privacy: Option<(RangeOrSet<usize>, Option<bool>)>,
    pub component: Option<String>,
    pub depends: Option<Vec<SetChange<u64>>>,
    pub duplicate_of: Option<u64>,
    pub flags: Option<Vec<Flag>>,
    pub groups: Option<Vec<SetChange<String>>>,
    pub keywords: Option<Vec<SetChange<String>>>,
    pub os: Option<String>,
    pub platform: Option<String>,
    pub priority: Option<String>,
    pub product: Option<String>,
    pub qa: Option<String>,
    pub resolution: Option<String>,
    pub see_also: Option<Vec<SetChange<String>>>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub target: Option<String>,
    pub url: Option<String>,
    pub version: Option<String>,
    pub whiteboard: Option<String>,

    #[serde(flatten)]
    pub custom_fields: Option<IndexMap<String, String>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    pub fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            alias: self.alias.or(other.alias),
            assignee: self.assignee.or(other.assignee),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            comment: self.comment.or(other.comment),
            comment_from: self.comment_from.or(other.comment_from),
            comment_is_private: self.comment_is_private.or(other.comment_is_private),
            comment_privacy: self.comment_privacy.or(other.comment_privacy),
            component: self.component.or(other.component),
            depends: self.depends.or(other.depends),
            duplicate_of: self.duplicate_of.or(other.duplicate_of),
            flags: self.flags.or(other.flags),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
            product: self.product.or(other.product),
            qa: self.qa.or(other.qa),
            resolution: self.resolution.or(other.resolution),
            see_also: self.see_also.or(other.see_also),
            status: self.status.or(other.status),
            severity: self.severity.or(other.severity),
            target: self.target.or(other.target),
            summary: self.summary.or(other.summary),
            url: self.url.or(other.url),
            version: self.version.or(other.version),
            whiteboard: self.whiteboard.or(other.whiteboard),

            custom_fields: self.custom_fields.or(other.custom_fields),
        }
    }

    /// Encode parameters into the form required for the request.
    async fn encode(
        self,
        service: &super::Service,
        ids: Vec<String>,
    ) -> crate::Result<RequestParameters> {
        let mut params = RequestParameters {
            alias: self.alias.map(|x| x.into_iter().collect()),
            blocks: self.blocks.map(|x| x.into_iter().collect()),
            component: self.component,
            depends_on: self.depends.map(|x| x.into_iter().collect()),
            dupe_of: self.duplicate_of,
            flags: self.flags,
            groups: self.groups.map(|x| x.into_iter().collect()),
            keywords: self.keywords.map(|x| x.into_iter().collect()),
            op_sys: self.os,
            platform: self.platform,
            priority: self.priority,
            product: self.product,
            resolution: self.resolution,
            severity: self.severity,
            status: self.status,
            summary: self.summary,
            target_milestone: self.target,
            url: self.url,
            version: self.version,
            whiteboard: self.whiteboard,

            // auto-prefix custom field names
            custom_fields: self.custom_fields.map(|values| {
                values
                    .into_iter()
                    .map(|(k, v)| {
                        if !k.starts_with("cf_") {
                            (format!("cf_{k}"), v)
                        } else {
                            (k, v)
                        }
                    })
                    .collect()
            }),

            ..Default::default()
        };

        if let Some(value) = self.assignee.as_ref() {
            if value.is_empty() {
                params.reset_assigned_to = Some(true);
            } else {
                let user = service.replace_user_alias(value);
                params.assigned_to = Some(user.into());
            }
        }

        if let Some(values) = self.cc {
            let iter = values.into_iter().map(|c| match c {
                SetChange::Add(value) => {
                    let user = service.replace_user_alias(&value);
                    SetChange::Add(user.into())
                }
                SetChange::Remove(value) => {
                    let user = service.replace_user_alias(&value);
                    SetChange::Remove(user.into())
                }
                SetChange::Set(value) => {
                    let user = service.replace_user_alias(&value);
                    SetChange::Set(user.into())
                }
            });

            params.cc = Some(iter.collect());
        }

        if let Some(value) = self.comment {
            params.comment = Some(Comment {
                body: value,
                is_private: self.comment_is_private.unwrap_or_default(),
            });
        } else if let Some(path) = self.comment_from.as_ref() {
            let data = fs::read_to_string(path).map_err(|e| {
                Error::InvalidValue(format!("failed reading comment file: {path}: {e}"))
            })?;
            params.comment = Some(Comment {
                body: data,
                is_private: self.comment_is_private.unwrap_or_default(),
            });
        }

        if let Some((value, is_private)) = self.comment_privacy {
            let id = match &ids[..] {
                [x] => x,
                _ => {
                    return Err(Error::InvalidValue(
                        "can't toggle comment privacy for multiple bugs".to_string(),
                    ))
                }
            };
            let comments = CommentRequest::new(service, &[id], None)?
                .send(service)
                .await?
                .into_iter()
                .next()
                .expect("invalid comments response");

            let mut toggled = IndexMap::new();
            for c in comments {
                if value.contains(&c.count) {
                    toggled.insert(c.id, is_private.unwrap_or(!c.is_private));
                }
            }

            params.comment_is_private = Some(toggled);
        }

        if let Some(value) = self.qa.as_ref() {
            if value.is_empty() {
                params.reset_qa_contact = Some(true);
            } else {
                let user = service.replace_user_alias(value);
                params.qa_contact = Some(user.into());
            }
        }

        if let Some(values) = self.see_also {
            // convert bug IDs to full URLs
            let iter = values.into_iter().map(|x| match x {
                SetChange::Add(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Add(service.item_url(value))
                }
                SetChange::Remove(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Remove(service.item_url(value))
                }
                SetChange::Set(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Set(service.item_url(value))
                }
                c => c,
            });

            params.see_also = Some(iter.collect());
        }

        // TODO: verify all required fields are non-empty
        if params == RequestParameters::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(params)
        }
    }
}

/// Internal bug update request parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct RequestParameters {
    alias: Option<SetChanges<String>>,
    assigned_to: Option<String>,
    blocks: Option<SetChanges<u64>>,
    cc: Option<Changes<String>>,
    comment: Option<Comment>,
    comment_is_private: Option<IndexMap<u64, bool>>,
    component: Option<String>,
    depends_on: Option<SetChanges<u64>>,
    dupe_of: Option<u64>,
    flags: Option<Vec<Flag>>,
    groups: Option<Changes<String>>,
    ids: Option<Vec<String>>,
    keywords: Option<SetChanges<String>>,
    op_sys: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    product: Option<String>,
    qa_contact: Option<String>,
    reset_assigned_to: Option<bool>,
    reset_qa_contact: Option<bool>,
    resolution: Option<String>,
    see_also: Option<Changes<String>>,
    severity: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    target_milestone: Option<String>,
    url: Option<String>,
    version: Option<String>,
    whiteboard: Option<String>,

    #[serde(flatten)]
    custom_fields: Option<IndexMap<String, String>>,
}
