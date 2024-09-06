use std::ops::{Deref, DerefMut};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString};
use tracing::debug;

use crate::objects::github::Issue;
use crate::query::{Order, Query};
use crate::traits::{Api, Merge, MergeOption, RequestSend, RequestTemplate};
use crate::utils::config_dir;

#[derive(Serialize, Debug)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a super::Service,
    #[serde(flatten)]
    pub params: Parameters,
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a super::Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(self.service);

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

impl RequestSend for Request<'_> {
    type Output = Vec<Issue>;

    async fn send(&self) -> crate::Result<Self::Output> {
        debug!("{:?}", self.params);
        let _params = self.encode()?;
        todo!("search requests unsupported")
    }
}

impl RequestTemplate for Request<'_> {
    type Template = Parameters;

    fn path(&self, name: &str) -> crate::Result<Utf8PathBuf> {
        if let Some(service_name) = self.service.config.name() {
            let path = format!("templates/{service_name}/search/{name}");
            config_dir().map(|x| x.join(path))
        } else {
            Ok(Utf8PathBuf::from(name))
        }
    }
}

struct QueryBuilder<'a> {
    _service: &'a super::Service,
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

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            order: self.order.merge(other.order),
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, Debug, Clone)]
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
