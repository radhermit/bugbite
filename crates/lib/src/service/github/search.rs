use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString};
use tracing::debug;

use crate::objects::github::Issue;
use crate::query::{Order, Query};
use crate::service::github::Github;
use crate::traits::{Api, Merge, MergeOption, RequestSend, RequestTemplate};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request {
    #[serde(skip)]
    service: Github,
    #[serde(flatten)]
    pub params: Parameters,
}

impl Request {
    pub(super) fn new(service: &Github) -> Self {
        Self {
            service: service.clone(),
            params: Default::default(),
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(&self.service);

        if let Some(value) = &self.params.order {
            query.insert("sort", value);
        }

        Ok(query)
    }

    pub fn order(mut self, value: Order<OrderField>) -> Self {
        self.params.order = Some(value);
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<Issue>;

    async fn send(&self) -> crate::Result<Self::Output> {
        debug!("{:?}", self.params);
        let _query = self.encode()?;
        todo!("search requests unsupported")
    }
}

impl RequestTemplate for Request {
    type Params = Parameters;
    type Service = Github;
    const TYPE: &'static str = "search";

    fn service(&self) -> &Self::Service {
        &self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

struct QueryBuilder<'a> {
    _service: &'a Github,
    query: Query,
}

impl Deref for QueryBuilder<'_> {
    type Target = Query;

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
    fn new(_service: &'a Github) -> Self {
        Self {
            _service,
            query: Default::default(),
        }
    }
}

/// Issue search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub order: Option<Order<OrderField>>,
}

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            order: self.order.merge(other.order),
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
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
