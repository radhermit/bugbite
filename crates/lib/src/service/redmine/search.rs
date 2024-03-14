use std::fmt;

use chrono::offset::Utc;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;

use crate::objects::redmine::Issue;
use crate::time::TimeDelta;
use crate::traits::{Query, Request, ServiceParams, WebService};
use crate::Error;

/// Construct a search query.
#[derive(Debug)]
pub struct QueryBuilder<'a> {
    _service: &'a super::Service,
    query: ListOrderedMultimap<String, String>,
}

impl<'a> ServiceParams<'a> for QueryBuilder<'a> {
    type Service = super::Service;

    fn new(_service: &'a Self::Service) -> Self {
        Self {
            _service,
            query: Default::default(),
        }
    }
}

impl QueryBuilder<'_> {
    pub fn ids<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.insert("issue_id", values.into_iter().join(","));
    }

    pub fn status(&mut self, value: &str) -> crate::Result<()> {
        // TODO: move valid status search values to an enum
        match value {
            "open" => self.append("status_id", "open"),
            "closed" => self.append("status_id", "closed"),
            "all" => self.append("status_id", "*"),
            _ => return Err(Error::InvalidValue(format!("invalid status: {value}"))),
        }
        Ok(())
    }

    pub fn created_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = format!(">={}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
        self.insert("created_on", target);
    }

    pub fn modified_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = format!(">={}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
        self.insert("updated_on", target);
    }

    pub fn summary(&mut self, value: &str) {
        self.insert("subject", format!("~{value}"));
    }

    pub fn extend<K, I, V>(&mut self, key: K, values: I)
    where
        I: IntoIterator<Item = V>,
        K: fmt::Display,
        V: fmt::Display,
    {
        for value in values {
            self.query.append(key.to_string(), value.to_string());
        }
    }

    pub fn append<K, V>(&mut self, key: K, value: V)
    where
        K: fmt::Display,
        V: fmt::Display,
    {
        self.query.append(key.to_string(), value.to_string());
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: fmt::Display,
        V: fmt::Display,
    {
        self.query.insert(key.to_string(), value.to_string());
    }
}

impl Query for QueryBuilder<'_> {
    fn params(&mut self) -> crate::Result<String> {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        // limit to open issues by default
        if !self.query.contains_key("status_id") {
            self.append("status_id", "open");
        }

        // most instances restrict queries to 100 results
        self.append("limit", 100);

        // sort by ascending issue ID by default
        if !self.query.contains_key("sort") {
            self.append("sort", "id:asc");
        }

        params.extend_pairs(self.query.iter());
        Ok(params.finish())
    }
}

#[derive(Debug)]
pub(crate) struct SearchRequest(url::Url);

impl SearchRequest {
    pub(super) fn new<Q: Query>(service: &super::Service, mut query: Q) -> crate::Result<Self> {
        let url = service
            .base()
            .join(&format!("issues.json?{}", query.params()?))?;
        Ok(Self(url))
    }
}

impl Request for SearchRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.0);
        let response = service.send(request).await?;
        let mut data = service.parse_response(response).await?;
        let data = data["issues"].take();
        Ok(serde_json::from_value(data)?)
    }
}
