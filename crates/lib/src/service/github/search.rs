use std::fs;
use std::ops::{Deref, DerefMut};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString, VariantNames};
use tracing::debug;

use crate::objects::github::Issue;
use crate::query::{self, Order};
use crate::traits::{Api, RequestMerge, RequestSend};
use crate::utils::or;
use crate::Error;

struct QueryBuilder<'a> {
    _service: &'a super::Service,
    query: query::QueryBuilder,
}

impl Deref for QueryBuilder<'_> {
    type Target = query::QueryBuilder;

    fn deref(&self) -> &Self::Target {
        &self.query
    }
}

impl DerefMut for QueryBuilder<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.query
    }
}

impl<'a> QueryBuilder<'a> {
    fn new(_service: &'a super::Service) -> Self {
        Self {
            _service,
            query: Default::default(),
        }
    }
}

/// Issue search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Parameters {
    pub order: Option<Order<OrderField>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    pub fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Override parameters using the provided value if it exists.
    fn merge<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        or!(self.order, other.order);
    }

    pub fn order(mut self, value: Order<OrderField>) -> Self {
        self.order = Some(value);
        self
    }

    pub(crate) fn encode(self, service: &super::Service) -> crate::Result<String> {
        let mut query = QueryBuilder::new(service);

        if let Some(value) = self.order {
            query.insert("sort", value);
        }

        Ok(query.encode())
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone)]
#[strum(serialize_all = "kebab-case")]
pub enum OrderField {
    Comments,
    Created,
    Interactions,
    Reactions,
    Updated,
}

impl Api for OrderField {
    fn api(&self) -> String {
        self.to_string()
    }
}

impl Api for Order<OrderField> {
    fn api(&self) -> String {
        match self {
            Order::Ascending(field) => field.api(),
            Order::Descending(field) => format!("-{}", field.api()),
        }
    }
}

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a super::Service,
    pub params: Parameters,
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a super::Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }
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
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        debug!("{:?}", self.params);
        let _params = self.params.encode(self.service)?;
        todo!("search requests unsupported")
    }
}
