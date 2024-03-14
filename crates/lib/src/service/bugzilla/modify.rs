use std::num::NonZeroU64;
use std::str::FromStr;
use std::{fmt, fs};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};

use crate::traits::{Request, WebService};
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
#[serde(deny_unknown_fields)]
struct Params {
    ids: Option<Vec<NonZeroU64>>,
    product: Option<String>,
    component: Option<String>,
    comment: Option<Comment>,
    status: Option<String>,
    resolution: Option<String>,
    dupe_of: Option<NonZeroU64>,
    summary: Option<String>,
    url: Option<String>,
    version: Option<String>,
    whiteboard: Option<String>,
    assigned_to: Option<String>,
    blocks: Option<SetChanges<NonZeroU64>>,
    depends_on: Option<SetChanges<NonZeroU64>>,
    cc: Option<Changes<String>>,
    groups: Option<Changes<String>>,
    keywords: Option<SetChanges<String>>,
}

/// Construct bug modification parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#update-bug for more
/// information.
pub struct ModifyParams(Params);

impl Default for ModifyParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ModifyParams {
    pub fn new() -> Self {
        Self(Params::default())
    }

    pub fn load(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        let params = toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))?;
        Ok(Self(params))
    }

    pub fn product(&mut self, value: &str) {
        self.0.product = Some(value.to_string());
    }

    pub fn component(&mut self, value: &str) {
        self.0.component = Some(value.to_string());
    }

    pub fn status(&mut self, value: &str) {
        self.0.status = Some(value.to_string());
    }

    pub fn resolution(&mut self, value: &str) {
        self.0.resolution = Some(value.to_string());
    }

    pub fn cc<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
    {
        self.0.cc = Some(values.into_iter().collect());
    }

    pub fn groups<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
    {
        self.0.groups = Some(values.into_iter().collect());
    }

    pub fn keywords<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<String>>,
    {
        self.0.keywords = Some(values.into_iter().collect());
    }

    pub fn assigned_to<S: Into<String>>(&mut self, value: S) {
        self.0.assigned_to = Some(value.into());
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<NonZeroU64>>,
    {
        self.0.blocks = Some(values.into_iter().collect());
    }

    pub fn depends_on<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Change<NonZeroU64>>,
    {
        self.0.depends_on = Some(values.into_iter().collect());
    }

    pub fn duplicate_of(&mut self, value: NonZeroU64) {
        self.0.dupe_of = Some(value);
    }

    pub fn summary(&mut self, value: &str) {
        self.0.summary = Some(value.to_string());
    }

    pub fn url(&mut self, value: &str) {
        self.0.url = Some(value.to_string());
    }

    pub fn version(&mut self, value: &str) {
        self.0.version = Some(value.to_string());
    }

    pub fn whiteboard(&mut self, value: &str) {
        self.0.whiteboard = Some(value.to_string());
    }

    pub fn comment(&mut self, value: &str) {
        let comment = Comment {
            body: value.to_string(),
            is_private: false,
        };
        self.0.comment = Some(comment);
    }

    fn build(self) -> crate::Result<Params> {
        if self.0 == Params::default() {
            Err(Error::EmptyParams)
        } else {
            Ok(self.0)
        }
    }
}
