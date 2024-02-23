use chrono::offset::Utc;
use ordered_multimap::ListOrderedMultimap;

use crate::objects::redmine::Issue;
use crate::time::TimeDelta;
use crate::traits::{Query, Request, WebService};

/// Construct a search query.
#[derive(Debug, Default)]
pub struct QueryBuilder {
    query: ListOrderedMultimap<String, String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
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
        K: ToString,
        V: ToString,
    {
        for value in values {
            self.query.append(key.to_string(), value.to_string());
        }
    }

    pub fn append<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
    {
        self.query.append(key.to_string(), value.to_string());
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: ToString,
        V: ToString,
    {
        self.query.insert(key.to_string(), value.to_string());
    }
}

impl Query for QueryBuilder {
    fn params(&mut self) -> crate::Result<String> {
        let mut params = url::form_urlencoded::Serializer::new(String::new());
        // TODO: limit to open issues by default?

        // most instances restrict queries to 100 results
        self.append("limit", 100);

        // return only open issues by default
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
        let response = service.client().get(self.0).send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["issues"].take();
        Ok(serde_json::from_value(data)?)
    }
}
