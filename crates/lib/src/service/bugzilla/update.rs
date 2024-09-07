use std::borrow::Cow;
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::{fmt, fs};

use camino::Utf8PathBuf;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumString};
use url::Url;

use crate::objects::{bugzilla::Flag, Range};
use crate::serde::non_empty_str;
use crate::service::bugzilla::Service;
use crate::traits::{
    Contains, InjectAuth, Merge, MergeOption, RequestSend, RequestTemplate, WebService,
};
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
    comment: Option<String>,
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

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(skip)]
    pub ids: Vec<String>,
    #[serde(flatten)]
    pub params: Parameters,
}

impl RequestSend for Request<'_> {
    type Output = Vec<BugChange>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.encode().await?;
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
        if let Some(comment) = &params.comment {
            for change in changes.iter_mut() {
                change.comment = Some(comment.body.to_string());
            }
        }
        Ok(changes)
    }
}

impl RequestTemplate for Request<'_> {
    type Params = Parameters;
    type Service = Service;
    const TYPE: &'static str = "update";

    fn service(&self) -> &Self::Service {
        self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
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

    /// Encode parameters into the form required for the request.
    async fn encode(&'a self) -> crate::Result<RequestParameters> {
        // verify parameters exist
        if self.params == Parameters::default() {
            return Err(Error::EmptyParams);
        }

        let mut params = RequestParameters {
            ids: &self.ids,
            alias: self.params.alias.as_ref().map(|x| x.iter().collect()),
            blocks: self.params.blocks.as_ref().map(|x| x.iter().collect()),
            component: self.params.component.as_deref(),
            depends_on: self.params.depends.as_ref().map(|x| x.iter().collect()),
            dupe_of: self.params.duplicate_of,
            flags: self.params.flags.as_deref(),
            groups: self.params.groups.as_ref().map(|x| x.iter().collect()),
            keywords: self.params.keywords.as_ref().map(|x| x.iter().collect()),
            op_sys: self.params.os.as_deref(),
            platform: self.params.platform.as_deref(),
            priority: self.params.priority.as_deref(),
            product: self.params.product.as_deref(),
            resolution: self.params.resolution.as_deref(),
            severity: self.params.severity.as_deref(),
            status: self.params.status.as_deref(),
            summary: self.params.summary.as_deref(),
            target_milestone: self.params.target.as_deref(),
            url: self.params.url.as_deref(),
            version: self.params.version.as_deref(),
            whiteboard: self.params.whiteboard.as_deref(),
            custom_fields: self.params.custom_fields.as_ref(),
            ..Default::default()
        };

        if let Some(value) = self.params.assignee.as_deref() {
            if value.is_empty() {
                params.reset_assigned_to = Some(true);
            } else {
                let user = self.service.replace_user_alias(value);
                params.assigned_to = Some(user);
            }
        }

        if let Some(values) = &self.params.cc {
            let iter = values.iter().map(|c| match c {
                SetChange::Add(value) => SetChange::Add(self.service.replace_user_alias(value)),
                SetChange::Remove(value) => {
                    SetChange::Remove(self.service.replace_user_alias(value))
                }
                SetChange::Set(value) => SetChange::Set(self.service.replace_user_alias(value)),
            });

            params.cc = Some(iter.collect());
        }

        if let Some(value) = self.params.comment.as_deref() {
            params.comment = Some(Comment {
                body: Cow::Borrowed(value),
                is_private: self.params.comment_is_private.unwrap_or_default(),
            });
        } else if let Some(path) = &self.params.comment_from {
            let data = fs::read_to_string(path).map_err(|e| {
                Error::InvalidValue(format!("failed reading comment file: {path}: {e}"))
            })?;
            if data.trim().is_empty() {
                return Err(Error::InvalidValue(format!("empty comment file: {path}")));
            }
            params.comment = Some(Comment {
                body: Cow::Owned(data),
                is_private: self.params.comment_is_private.unwrap_or_default(),
            });
        }

        if let Some((value, is_private)) = &self.params.comment_privacy {
            if params.ids.len() > 1 {
                return Err(Error::InvalidValue(
                    "can't toggle comment privacy for multiple bugs".to_string(),
                ));
            }

            // pull comments for a given bug ID
            let comments = self
                .service
                .comment([&params.ids[0]])
                .send()
                .await?
                .into_iter()
                .next()
                .expect("invalid comments response");

            for c in comments {
                if value.contains(&c.count) {
                    params
                        .comment_is_private
                        .get_or_insert_with(Default::default)
                        .insert(c.id, is_private.unwrap_or(!c.is_private));
                }
            }
        }

        if let Some(value) = self.params.qa.as_deref() {
            if value.is_empty() {
                params.reset_qa_contact = Some(true);
            } else {
                let user = self.service.replace_user_alias(value);
                params.qa_contact = Some(user);
            }
        }

        if let Some(values) = &self.params.see_also {
            // convert bug IDs to full URLs
            let parse = |value: &'a str| -> Cow<'a, str> {
                if let Ok(id) = value.parse::<u64>() {
                    Cow::Owned(self.service.item_url(id))
                } else {
                    Cow::Borrowed(value)
                }
            };

            let iter = values.iter().map(|x| match x {
                SetChange::Add(value) => SetChange::Add(parse(value)),
                SetChange::Remove(value) => SetChange::Remove(parse(value)),
                SetChange::Set(value) => SetChange::Set(parse(value)),
            });

            params.see_also = Some(iter.collect());
        }

        Ok(params)
    }
}

/// Supported change variants for set-based fields.
#[derive(DeserializeFromStr, SerializeDisplay, Debug, Eq, PartialEq, Clone)]
pub enum SetChange<T> {
    Add(T),
    Remove(T),
    Set(T),
}

impl<T: FromStr> FromStr for SetChange<T> {
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

impl<T: fmt::Display> fmt::Display for SetChange<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Add(value) => write!(f, "+{value}"),
            Self::Remove(value) => write!(f, "-{value}"),
            Self::Set(value) => value.fmt(f),
        }
    }
}

/// Tri-state boolean logic that supports (de)serialization.
#[derive(
    DeserializeFromStr, SerializeDisplay, Display, EnumString, Debug, PartialEq, Eq, Clone, Copy,
)]
#[strum(serialize_all = "kebab-case")]
pub enum TriBool {
    False,
    True,
    None,
}

impl Deref for TriBool {
    type Target = Option<bool>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::False => &Some(false),
            Self::True => &Some(true),
            Self::None => &None,
        }
    }
}

/// Bug update parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub alias: Option<Vec<SetChange<String>>>,
    pub assignee: Option<String>,
    pub blocks: Option<Vec<SetChange<u64>>>,
    pub cc: Option<Vec<SetChange<String>>>,
    pub comment: Option<String>,
    pub comment_from: Option<Utf8PathBuf>,
    pub comment_is_private: Option<bool>,
    pub comment_privacy: Option<(RangeOrSet<usize>, TriBool)>,
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

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            alias: self.alias.merge(other.alias),
            assignee: self.assignee.merge(other.assignee),
            blocks: self.blocks.merge(other.blocks),
            cc: self.cc.merge(other.cc),
            comment: self.comment.merge(other.comment),
            comment_from: self.comment_from.merge(other.comment_from),
            comment_is_private: self.comment_is_private.merge(other.comment_is_private),
            comment_privacy: self.comment_privacy.merge(other.comment_privacy),
            component: self.component.merge(other.component),
            depends: self.depends.merge(other.depends),
            duplicate_of: self.duplicate_of.merge(other.duplicate_of),
            flags: self.flags.merge(other.flags),
            groups: self.groups.merge(other.groups),
            keywords: self.keywords.merge(other.keywords),
            os: self.os.merge(other.os),
            platform: self.platform.merge(other.platform),
            priority: self.priority.merge(other.priority),
            product: self.product.merge(other.product),
            qa: self.qa.merge(other.qa),
            resolution: self.resolution.merge(other.resolution),
            see_also: self.see_also.merge(other.see_also),
            status: self.status.merge(other.status),
            severity: self.severity.merge(other.severity),
            target: self.target.merge(other.target),
            summary: self.summary.merge(other.summary),
            url: self.url.merge(other.url),
            version: self.version.merge(other.version),
            whiteboard: self.whiteboard.merge(other.whiteboard),
            custom_fields: self.custom_fields.merge(other.custom_fields),
        }
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
struct SetChanges<T> {
    add: Option<Vec<T>>,
    remove: Option<Vec<T>>,
    set: Option<Vec<T>>,
}

impl<'a, T: FromStr> FromIterator<&'a SetChange<T>> for SetChanges<&'a T> {
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
#[derive(Serialize)]
struct Changes<T> {
    add: Option<Vec<T>>,
    remove: Option<Vec<T>>,
}

impl<T> FromIterator<SetChange<T>> for Changes<T> {
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

impl<'a, T> FromIterator<&'a SetChange<T>> for Changes<&'a T> {
    fn from_iter<I: IntoIterator<Item = &'a SetChange<T>>>(iterable: I) -> Self {
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

/// Comment field for bug update request parameters.
#[derive(Serialize)]
struct Comment<'a> {
    body: Cow<'a, str>,
    is_private: bool,
}

/// Internal bug update request parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
#[skip_serializing_none]
#[derive(Serialize, Default)]
struct RequestParameters<'a> {
    ids: &'a [String],
    alias: Option<SetChanges<&'a String>>,
    assigned_to: Option<&'a str>,
    blocks: Option<SetChanges<&'a u64>>,
    cc: Option<Changes<&'a str>>,
    comment: Option<Comment<'a>>,
    comment_is_private: Option<IndexMap<u64, bool>>,
    component: Option<&'a str>,
    depends_on: Option<SetChanges<&'a u64>>,
    dupe_of: Option<u64>,
    flags: Option<&'a [Flag]>,
    groups: Option<Changes<&'a String>>,
    keywords: Option<SetChanges<&'a String>>,
    op_sys: Option<&'a str>,
    platform: Option<&'a str>,
    priority: Option<&'a str>,
    product: Option<&'a str>,
    qa_contact: Option<&'a str>,
    reset_assigned_to: Option<bool>,
    reset_qa_contact: Option<bool>,
    resolution: Option<&'a str>,
    see_also: Option<Changes<Cow<'a, str>>>,
    severity: Option<&'a str>,
    status: Option<&'a str>,
    summary: Option<&'a str>,
    target_milestone: Option<&'a str>,
    url: Option<&'a str>,
    version: Option<&'a str>,
    whiteboard: Option<&'a str>,

    #[serde(flatten)]
    custom_fields: Option<&'a IndexMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let service = Service::new(server.uri()).unwrap();

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
