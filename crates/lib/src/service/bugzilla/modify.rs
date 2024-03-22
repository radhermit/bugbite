use std::num::NonZeroU64;
use std::str::FromStr;
use std::{fmt, fs};

use camino::Utf8Path;
use indexmap::IndexMap;
use itertools::Either;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::serde::non_empty_str;
use crate::traits::{InjectAuth, Request, ServiceParams, WebService};
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
    id: NonZeroU64,
    comment: Option<Comment>,
    changes: IndexMap<String, FieldChange>,
}

impl fmt::Display for BugChange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "=== Bug #{} ===", self.id)?;
        write!(f, "--- Modified fields ---")?;
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

#[derive(Debug)]
pub(crate) struct ModifyRequest {
    url: url::Url,
    params: Params,
}

impl Request for ModifyRequest {
    type Output = Vec<BugChange>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service
            .client()
            .put(self.url)
            .json(&self.params)
            .inject_auth(service, true)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        let mut changes: Vec<BugChange> = serde_json::from_value(data)?;
        if let Some(comment) = self.params.comment.as_ref() {
            for change in changes.iter_mut() {
                change.comment = Some(comment.clone());
            }
        }
        Ok(changes)
    }
}

impl ModifyRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        params: ModifyParams,
    ) -> crate::Result<Self>
    where
        S: fmt::Display,
    {
        let [id, ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut params = params.build()?;
        params.ids = Some(ids.iter().map(|x| x.to_string()).collect());

        Ok(Self {
            url: service.base().join(&format!("rest/bug/{id}"))?,
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

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Params {
    alias: Option<String>,
    assigned_to: Option<String>,
    blocks: Option<SetChanges<NonZeroU64>>,
    cc: Option<Changes<String>>,
    comment: Option<Comment>,
    comment_is_private: Option<IndexMap<u64, bool>>,
    component: Option<String>,
    depends_on: Option<SetChanges<NonZeroU64>>,
    dupe_of: Option<NonZeroU64>,
    groups: Option<Changes<String>>,
    ids: Option<Vec<String>>,
    keywords: Option<SetChanges<String>>,
    op_sys: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    product: Option<String>,
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

/// Construct bug modification parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
pub struct ModifyParams<'a> {
    service: &'a super::Service,
    params: Params,
}

impl<'a> ServiceParams<'a> for ModifyParams<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }
}

impl<'a> ModifyParams<'a> {
    pub fn load(path: &Utf8Path, service: &'a super::Service) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))?;
        Ok(Self { service, params })
    }

    fn build(self) -> crate::Result<Params> {
        if self.params == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.params)
        }
    }

    pub fn alias(&mut self, value: &str) {
        self.params.alias = Some(value.into());
    }

    pub fn assigned_to(&mut self, value: &str) {
        // TODO: support pulling aliases from the config?
        if value == "@me" {
            if let Some(user) = self.service.user() {
                self.params.assigned_to = Some(user.into());
            }
        } else {
            self.params.assigned_to = Some(value.into());
        }
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<NonZeroU64>>,
    {
        self.params.blocks = Some(values.into_iter().collect());
    }

    pub fn cc<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        // TODO: support pulling aliases from the config?
        let iter = if let Some(user) = self.service.user() {
            Either::Left(values.into_iter().map(|c| match c {
                SetChange::Add(value) if value == "@me" => SetChange::Add(user.into()),
                SetChange::Remove(value) if value == "@me" => SetChange::Remove(user.into()),
                SetChange::Set(value) if value == "@me" => SetChange::Set(user.into()),
                c => c,
            }))
        } else {
            Either::Right(values.into_iter())
        };

        self.params.cc = Some(iter.collect());
    }

    pub fn comment(&mut self, value: &str) {
        let comment = Comment {
            body: value.into(),
            is_private: false,
        };
        self.params.comment = Some(comment);
    }

    pub fn component(&mut self, value: &str) {
        self.params.component = Some(value.into());
    }

    pub fn depends_on<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<NonZeroU64>>,
    {
        self.params.depends_on = Some(values.into_iter().collect());
    }

    pub fn duplicate_of(&mut self, value: NonZeroU64) {
        self.params.dupe_of = Some(value);
    }

    pub fn custom_fields<I, K, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        self.params.custom_fields = Some(
            values
                .into_iter()
                .map(|(k, v)| match k.as_ref() {
                    k if k.starts_with("cf_") => (k.into(), v.into()),
                    k => (format!("cf_{k}"), v.into()),
                })
                .collect(),
        );
    }

    pub fn groups<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        self.params.groups = Some(values.into_iter().collect());
    }

    pub fn keywords<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        self.params.keywords = Some(values.into_iter().collect());
    }

    pub fn os(&mut self, value: &str) {
        self.params.op_sys = Some(value.into());
    }

    pub fn platform(&mut self, value: &str) {
        self.params.platform = Some(value.into());
    }

    pub fn priority(&mut self, value: &str) {
        self.params.priority = Some(value.into());
    }

    pub fn private_comments<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (u64, bool)>,
    {
        self.params.comment_is_private = Some(values.into_iter().collect());
    }

    pub fn product(&mut self, value: &str) {
        self.params.product = Some(value.into());
    }

    pub fn resolution(&mut self, value: &str) {
        self.params.resolution = Some(value.into());
    }

    pub fn see_also<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        self.params.see_also = Some(values.into_iter().collect());
    }

    pub fn severity(&mut self, value: &str) {
        self.params.severity = Some(value.into());
    }

    pub fn status(&mut self, value: &str) {
        self.params.status = Some(value.into());
    }

    pub fn summary(&mut self, value: &str) {
        self.params.summary = Some(value.into());
    }

    pub fn target(&mut self, value: &str) {
        self.params.target_milestone = Some(value.into());
    }

    pub fn url(&mut self, value: &str) {
        self.params.url = Some(value.into());
    }

    pub fn version(&mut self, value: &str) {
        self.params.version = Some(value.into());
    }

    pub fn whiteboard(&mut self, value: &str) {
        self.params.whiteboard = Some(value.into());
    }
}
