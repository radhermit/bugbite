use std::collections::HashMap;
use std::num::NonZeroU64;
use std::str::FromStr;
use std::{fmt, fs};

use camino::Utf8Path;
use itertools::Either;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::traits::{Request, ServiceParams, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct ModifyRequest {
    url: url::Url,
    params: Params,
}

impl Request for ModifyRequest {
    type Output = ();
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().put(self.url).json(&self.params);
        let response = service.send(request).await?;
        let mut data = service.parse_response(response).await?;
        let _data = data["bugs"].take();
        Ok(())
    }
}

impl ModifyRequest {
    pub(super) fn new(
        service: &super::Service,
        ids: &[NonZeroU64],
        params: ModifyParams,
    ) -> crate::Result<Self> {
        let [id, ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut params = params.build()?;
        params.ids = Some(ids.to_vec());

        Ok(Self {
            url: service.base().join(&format!("rest/bug/{id}"))?,
            params,
        })
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Eq, PartialEq, Clone)]
pub enum Change<T: Clone> {
    Add(T),
    Remove(T),
    Set(T),
}

impl<T: FromStr + Clone> FromStr for Change<T> {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        if let Some(value) = s.strip_prefix('+') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Change::Add(value))
        } else if let Some(value) = s.strip_prefix('-') {
            let value = value
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Change::Remove(value))
        } else {
            let value = s
                .parse()
                .map_err(|_| Error::InvalidValue(format!("failed parsing change: {s}")))?;
            Ok(Change::Set(value))
        }
    }
}

impl<T: FromStr + Clone + fmt::Display> fmt::Display for Change<T> {
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

impl<T: FromStr + Clone> FromIterator<Change<T>> for SetChanges<T> {
    fn from_iter<I: IntoIterator<Item = Change<T>>>(iterable: I) -> Self {
        let (mut add, mut remove, mut set) = (vec![], vec![], vec![]);
        for change in iterable {
            match change {
                Change::Add(value) => add.push(value),
                Change::Remove(value) => remove.push(value),
                Change::Set(value) => set.push(value),
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

impl<T: FromStr + Clone> FromIterator<Change<T>> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = Change<T>>>(iterable: I) -> Self {
        let (mut add, mut remove) = (vec![], vec![]);
        for change in iterable {
            match change {
                Change::Add(value) | Change::Set(value) => add.push(value),
                Change::Remove(value) => remove.push(value),
            }
        }

        Self {
            add: Some(add),
            remove: Some(remove),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
struct Comment {
    body: String,
    is_private: bool,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Eq, PartialEq)]
struct Params {
    assigned_to: Option<String>,
    blocks: Option<SetChanges<NonZeroU64>>,
    cc: Option<Changes<String>>,
    comment: Option<Comment>,
    component: Option<String>,
    depends_on: Option<SetChanges<NonZeroU64>>,
    dupe_of: Option<NonZeroU64>,
    groups: Option<Changes<String>>,
    ids: Option<Vec<NonZeroU64>>,
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
    custom_fields: Option<HashMap<String, String>>,
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

    pub fn assigned_to(&mut self, value: &str) {
        self.params.assigned_to = Some(value.into());
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<NonZeroU64>>,
    {
        self.params.blocks = Some(values.into_iter().collect());
    }

    pub fn cc<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
    {
        // replace @me alias with current service user if one exists
        let iter = if let Some(user) = self.service.user() {
            Either::Left(values.into_iter().map(|c| match c {
                Change::Add(value) if value == "@me" => Change::Add(user.into()),
                Change::Remove(value) if value == "@me" => Change::Remove(user.into()),
                Change::Set(value) if value == "@me" => Change::Set(user.into()),
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
        I: IntoIterator<Item = Change<NonZeroU64>>,
    {
        self.params.depends_on = Some(values.into_iter().collect());
    }

    pub fn duplicate_of(&mut self, value: NonZeroU64) {
        self.params.dupe_of = Some(value);
    }

    pub fn custom_fields<I, K, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.params.custom_fields = Some(
            values
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
    }

    pub fn groups<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
    {
        self.params.groups = Some(values.into_iter().collect());
    }

    pub fn keywords<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
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

    pub fn product(&mut self, value: &str) {
        self.params.product = Some(value.into());
    }

    pub fn resolution(&mut self, value: &str) {
        self.params.resolution = Some(value.into());
    }

    pub fn see_also<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
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
