use std::fmt;
use std::str::FromStr;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::objects::bugzilla::Flag;
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
    id: u64,
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
pub(crate) struct ModifyRequest<'a> {
    url: url::Url,
    params: Params,
    service: &'a super::Service,
}

impl Request for ModifyRequest<'_> {
    type Output = Vec<BugChange>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .put(self.url)
            .json(&self.params)
            .inject_auth(self.service, true)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let mut changes: Vec<BugChange> = serde_json::from_value(data)
            .map_err(|e| Error::InvalidValue(format!("failed deserializing changes: {e}")))?;
        if let Some(comment) = self.params.comment.as_ref() {
            for change in changes.iter_mut() {
                change.comment = Some(comment.clone());
            }
        }
        Ok(changes)
    }
}

impl<'a> ModifyRequest<'a> {
    pub(super) fn new<S>(
        service: &'a super::Service,
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
            service,
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
    fn build(self) -> crate::Result<Params> {
        if self.params == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.params)
        }
    }

    pub fn alias<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        self.params.alias = Some(values.into_iter().collect());
    }

    pub fn assignee(&mut self, value: Option<&str>) {
        if let Some(name) = value {
            let user = self.service.replace_user_alias(name);
            self.params.assigned_to = Some(user.into());
        } else {
            self.params.reset_assigned_to = Some(true);
        }
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<u64>>,
    {
        self.params.blocks = Some(values.into_iter().collect());
    }

    pub fn cc<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        let iter = values.into_iter().map(|c| match c {
            SetChange::Add(value) => {
                let user = self.service.replace_user_alias(&value);
                SetChange::Add(user.into())
            }
            SetChange::Remove(value) => {
                let user = self.service.replace_user_alias(&value);
                SetChange::Remove(user.into())
            }
            SetChange::Set(value) => {
                let user = self.service.replace_user_alias(&value);
                SetChange::Set(user.into())
            }
        });

        self.params.cc = Some(iter.collect());
    }

    pub fn comment<S: Into<String>>(&mut self, value: S, is_private: bool) {
        let comment = Comment {
            body: value.into(),
            is_private,
        };
        self.params.comment = Some(comment);
    }

    pub fn component<S: Into<String>>(&mut self, value: S) {
        self.params.component = Some(value.into());
    }

    pub fn depends<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<u64>>,
    {
        self.params.depends_on = Some(values.into_iter().collect());
    }

    pub fn duplicate_of(&mut self, value: u64) {
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

    pub fn flags<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Flag>,
    {
        self.params.flags = Some(values.into_iter().collect());
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

    pub fn os<S: Into<String>>(&mut self, value: S) {
        self.params.op_sys = Some(value.into());
    }

    pub fn platform<S: Into<String>>(&mut self, value: S) {
        self.params.platform = Some(value.into());
    }

    pub fn priority<S: Into<String>>(&mut self, value: S) {
        self.params.priority = Some(value.into());
    }

    pub fn comment_is_private<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (u64, bool)>,
    {
        self.params.comment_is_private = Some(values.into_iter().collect());
    }

    pub fn product<S: Into<String>>(&mut self, value: S) {
        self.params.product = Some(value.into());
    }

    pub fn qa(&mut self, value: Option<&str>) {
        if let Some(name) = value {
            let user = self.service.replace_user_alias(name);
            self.params.qa_contact = Some(user.into());
        } else {
            self.params.reset_qa_contact = Some(true);
        }
    }

    pub fn resolution<S: Into<String>>(&mut self, value: S) {
        self.params.resolution = Some(value.into());
    }

    pub fn see_also<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        // convert bug IDs to full URLs
        let iter = values.into_iter().map(|x| match x {
            SetChange::Add(value) if value.parse::<u64>().is_ok() => {
                SetChange::Add(self.service.item_url(value))
            }
            SetChange::Remove(value) if value.parse::<u64>().is_ok() => {
                SetChange::Remove(self.service.item_url(value))
            }
            SetChange::Set(value) if value.parse::<u64>().is_ok() => {
                SetChange::Set(self.service.item_url(value))
            }
            c => c,
        });

        self.params.see_also = Some(iter.collect());
    }

    pub fn severity<S: Into<String>>(&mut self, value: S) {
        self.params.severity = Some(value.into());
    }

    pub fn status<S: Into<String>>(&mut self, value: S) {
        self.params.status = Some(value.into());
    }

    pub fn summary<S: Into<String>>(&mut self, value: S) {
        self.params.summary = Some(value.into());
    }

    pub fn target<S: Into<String>>(&mut self, value: S) {
        self.params.target_milestone = Some(value.into());
    }

    pub fn url<S: Into<String>>(&mut self, value: S) {
        self.params.url = Some(value.into());
    }

    pub fn version<S: Into<String>>(&mut self, value: S) {
        self.params.version = Some(value.into());
    }

    pub fn whiteboard<S: Into<String>>(&mut self, value: S) {
        self.params.whiteboard = Some(value.into());
    }
}
