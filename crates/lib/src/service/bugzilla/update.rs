use std::hash::Hash;
use std::str::FromStr;
use std::{fmt, fs};

use camino::{Utf8Path, Utf8PathBuf};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use url::Url;

use crate::objects::{bugzilla::Flag, Range};
use crate::serde::non_empty_str;
use crate::service::bugzilla::Service;
use crate::traits::{Contains, InjectAuth, RequestMerge, RequestSend, WebService};
use crate::utils::{or, prefix};
use crate::Error;

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

#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
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

#[derive(Serialize, Debug)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(skip)]
    pub ids: Vec<String>,
    #[serde(flatten)]
    pub params: Parameters,
}

impl RequestMerge<&Utf8Path> for Request<'_> {
    fn merge(&mut self, path: &Utf8Path) -> crate::Result<()> {
        let params = Parameters::from_path(path)?;
        self.params.merge(params);
        Ok(())
    }
}

impl<T: Into<Parameters>> RequestMerge<T> for Request<'_> {
    fn merge(&mut self, value: T) -> crate::Result<()> {
        self.params.merge(value);
        Ok(())
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<BugChange>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.params.encode(self.service, &self.ids).await?;
        let request = self
            .service
            .client
            .put(url)
            .json(&params)
            .auth(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let mut changes: Vec<BugChange> = serde_json::from_value(data)
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing changes: {e}")))?;
        if let Some(comment) = params.comment.as_ref() {
            for change in changes.iter_mut() {
                change.comment = Some(comment.clone());
            }
        }
        Ok(changes)
    }
}

impl<'a> Request<'a> {
    pub(super) fn new<I, S>(service: &'a Service, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        Self {
            service,
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            params: Default::default(),
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;
        let url = self.service.config.base.join(&format!("rest/bug/{id}"))?;
        Ok(url)
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

impl<'a, T: FromStr + Clone> FromIterator<&'a SetChange<T>> for SetChanges<&'a T> {
    fn from_iter<I: IntoIterator<Item = &'a SetChange<T>>>(iterable: I) -> Self {
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
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Clone)]
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
    fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Override parameters using the provided value if it exists.
    fn merge<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        or!(self.alias, other.alias);
        or!(self.assignee, other.assignee);
        or!(self.blocks, other.blocks);
        or!(self.cc, other.cc);
        or!(self.comment, other.comment);
        or!(self.comment_from, other.comment_from);
        or!(self.comment_is_private, other.comment_is_private);
        or!(self.comment_privacy, other.comment_privacy);
        or!(self.component, other.component);
        or!(self.depends, other.depends);
        or!(self.duplicate_of, other.duplicate_of);
        or!(self.flags, other.flags);
        or!(self.groups, other.groups);
        or!(self.keywords, other.keywords);
        or!(self.os, other.os);
        or!(self.platform, other.platform);
        or!(self.priority, other.priority);
        or!(self.product, other.product);
        or!(self.qa, other.qa);
        or!(self.resolution, other.resolution);
        or!(self.see_also, other.see_also);
        or!(self.status, other.status);
        or!(self.severity, other.severity);
        or!(self.target, other.target);
        or!(self.summary, other.summary);
        or!(self.url, other.url);
        or!(self.version, other.version);
        or!(self.whiteboard, other.whiteboard);
        or!(self.custom_fields, other.custom_fields);
    }

    /// Encode parameters into the form required for the request.
    async fn encode<'a>(
        &'a self,
        service: &Service,
        ids: &'a [String],
    ) -> crate::Result<RequestParameters<'a>> {
        // verify parameters exist
        if self == &Self::default() {
            return Err(Error::EmptyParams);
        }

        let mut params = RequestParameters {
            ids,
            alias: self.alias.as_ref().map(|x| x.iter().collect()),
            blocks: self.blocks.as_ref().map(|x| x.iter().collect()),
            component: self.component.as_deref(),
            depends_on: self.depends.as_ref().map(|x| x.iter().collect()),
            dupe_of: self.duplicate_of,
            flags: self.flags.as_deref(),
            groups: self
                .groups
                .as_ref()
                .map(|x| x.clone().into_iter().collect()),
            keywords: self.keywords.as_ref().map(|x| x.iter().collect()),
            op_sys: self.os.as_deref(),
            platform: self.platform.as_deref(),
            priority: self.priority.as_deref(),
            product: self.product.as_deref(),
            resolution: self.resolution.as_deref(),
            severity: self.severity.as_deref(),
            status: self.status.as_deref(),
            summary: self.summary.as_deref(),
            target_milestone: self.target.as_deref(),
            url: self.url.as_deref(),
            version: self.version.as_deref(),
            whiteboard: self.whiteboard.as_deref(),

            // auto-prefix custom field names
            custom_fields: self.custom_fields.as_ref().map(|values| {
                values
                    .into_iter()
                    .map(|(name, value)| (prefix!("cf_", name), value))
                    .collect()
            }),

            ..Default::default()
        };

        if let Some(value) = self.assignee.as_ref() {
            if value.is_empty() {
                params.reset_assigned_to = Some(true);
            } else {
                let user = service.replace_user_alias(value);
                params.assigned_to = Some(user);
            }
        }

        if let Some(values) = &self.cc {
            let iter = values.iter().map(|c| match c {
                SetChange::Add(value) => SetChange::Add(service.replace_user_alias(value)),
                SetChange::Remove(value) => SetChange::Remove(service.replace_user_alias(value)),
                SetChange::Set(value) => SetChange::Set(service.replace_user_alias(value)),
            });

            params.cc = Some(iter.collect());
        }

        if let Some(value) = &self.comment {
            params.comment = Some(Comment {
                body: value.clone(),
                is_private: self.comment_is_private.unwrap_or_default(),
            });
        } else if let Some(path) = &self.comment_from {
            let data = fs::read_to_string(path).map_err(|e| {
                Error::InvalidValue(format!("failed reading comment file: {path}: {e}"))
            })?;
            params.comment = Some(Comment {
                body: data,
                is_private: self.comment_is_private.unwrap_or_default(),
            });
        }

        if let Some((value, is_private)) = &self.comment_privacy {
            let id = match params.ids {
                [x] => x,
                _ => {
                    return Err(Error::InvalidValue(
                        "can't toggle comment privacy for multiple bugs".to_string(),
                    ))
                }
            };
            let comments = service
                .comment([id])
                .send()
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
                params.qa_contact = Some(user);
            }
        }

        if let Some(values) = &self.see_also {
            // convert bug IDs to full URLs
            let iter = values.iter().map(|x| match x {
                SetChange::Add(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Add(service.item_url(value))
                }
                SetChange::Remove(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Remove(service.item_url(value))
                }
                SetChange::Set(value) if value.parse::<u64>().is_ok() => {
                    SetChange::Set(service.item_url(value))
                }
                c => c.clone(),
            });

            params.see_also = Some(iter.collect());
        }

        Ok(params)
    }
}

/// Internal bug update request parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
#[skip_serializing_none]
#[derive(Serialize, Default, Eq, PartialEq)]
struct RequestParameters<'a> {
    ids: &'a [String],
    alias: Option<SetChanges<&'a String>>,
    assigned_to: Option<String>,
    blocks: Option<SetChanges<&'a u64>>,
    cc: Option<Changes<String>>,
    comment: Option<Comment>,
    comment_is_private: Option<IndexMap<u64, bool>>,
    component: Option<&'a str>,
    depends_on: Option<SetChanges<&'a u64>>,
    dupe_of: Option<u64>,
    flags: Option<&'a [Flag]>,
    groups: Option<Changes<String>>,
    keywords: Option<SetChanges<&'a String>>,
    op_sys: Option<&'a str>,
    platform: Option<&'a str>,
    priority: Option<&'a str>,
    product: Option<&'a str>,
    qa_contact: Option<String>,
    reset_assigned_to: Option<bool>,
    reset_qa_contact: Option<bool>,
    resolution: Option<&'a str>,
    see_also: Option<Changes<String>>,
    severity: Option<&'a str>,
    status: Option<&'a str>,
    summary: Option<&'a str>,
    target_milestone: Option<&'a str>,
    url: Option<&'a str>,
    version: Option<&'a str>,
    whiteboard: Option<&'a str>,

    #[serde(flatten)]
    custom_fields: Option<IndexMap<String, &'a String>>,
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // empty params
        let err = service.update([1]).send().await.unwrap_err();
        assert!(matches!(err, Error::EmptyParams));
    }
}
