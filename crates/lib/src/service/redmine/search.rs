use std::ops::{Deref, DerefMut};
use std::{fmt, fs};

use camino::Utf8Path;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::args::ExistsOrValues;
use crate::objects::redmine::Issue;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, Query};
use crate::service::redmine::Service;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{Api, InjectAuth, RequestMerge, RequestSend, RequestStream, WebService};
use crate::utils::or;
use crate::Error;

#[derive(Serialize, Debug, Clone)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(flatten)]
    pub params: Parameters,
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(self.service);

        if let Some(values) = &self.params.blocks {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocks, *value),
                ExistsOrValues::Values(values) => query.insert("blocks", values.iter().join(",")),
            }
        }

        if let Some(values) = &self.params.blocked {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocked, *value),
                ExistsOrValues::Values(values) => query.insert("blocked", values.iter().join(",")),
            }
        }

        if let Some(values) = &self.params.relates {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Relates, *value),
                ExistsOrValues::Values(values) => query.insert("relates", values.iter().join(",")),
            }
        }

        if let Some(values) = &self.params.ids {
            query.id(values)?;
        }

        if let Some(value) = &self.params.closed {
            query.time("closed_on", value);
        }

        if let Some(value) = &self.params.created {
            query.time("created_on", value);
        }

        if let Some(value) = &self.params.updated {
            query.time("updated_on", value);
        }

        if let Some(value) = &self.params.assignee {
            query.exists(ExistsField::Assignee, *value);
        }

        if let Some(values) = &self.params.attachments {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Attachment, *value),
                ExistsOrValues::Values(values) => {
                    let value = quoted_strings(values);
                    // TODO: support other operators, currently this specifies the `contains` op
                    query.insert("attachment", format!("~{value}"));
                }
            }
        }

        if let Some(values) = &self.params.summary {
            let value = quoted_strings(values);
            // TODO: support other operators, currently this specifies the `contains` op
            query.insert("subject", format!("~{value}"));
        }

        // limit to open issues by default
        query.status(self.params.status.as_deref().unwrap_or("@open"))?;

        if let Some(values) = &self.params.order {
            let value = values.iter().map(|x| x.api()).join(",");
            query.insert("sort", value);
        } else {
            // sort by ascending ID by default
            query.insert("sort", Order::Ascending(OrderField::Id));
        }

        if let Some(value) = &self.params.limit {
            query.insert("limit", value);
        }

        if let Some(value) = &self.params.offset {
            query.insert("offset", value);
        }

        Ok(query)
    }

    /// Return the website URL for a query.
    pub fn search_url(self) -> crate::Result<Url> {
        let mut url = self.service.config.base.join("issues?set_filter=1")?;
        let params = self.encode()?;
        url.query_pairs_mut().extend_pairs(&params.query);
        Ok(url)
    }

    pub fn order<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = Order<OrderField>>,
    {
        self.params.order = Some(values.into_iter().collect());
        self
    }

    pub fn status<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.status = Some(value.into());
        self
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

    async fn send(&self) -> crate::Result<Self::Output> {
        let mut url = self.service.config.base.join("issues.json")?;
        let params = self.encode()?;
        url.query_pairs_mut().extend_pairs(&params.query);
        let request = self.service.client.get(url).auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["issues"].take();
        serde_json::from_value(data)
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing issues: {e}")))
    }
}

impl RequestStream for Request<'_> {
    type Item = Issue;

    fn max_search_results(&self) -> usize {
        self.service.config.max_search_results
    }

    fn limit(&self) -> Option<usize> {
        self.params.limit
    }

    fn set_limit(&mut self, value: usize) {
        self.params.limit = Some(value);
    }

    fn offset(&self) -> Option<usize> {
        self.params.offset
    }

    fn set_offset(&mut self, value: usize) {
        self.params.offset = Some(value);
    }
}

/// Issue search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Parameters {
    pub assignee: Option<bool>,
    pub attachments: Option<ExistsOrValues<String>>,
    pub blocks: Option<ExistsOrValues<u64>>,
    pub blocked: Option<ExistsOrValues<u64>>,
    pub relates: Option<ExistsOrValues<u64>>,
    pub ids: Option<Vec<RangeOrValue<u64>>>,

    pub created: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub updated: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub closed: Option<RangeOrValue<TimeDeltaOrStatic>>,

    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: Option<Vec<Order<OrderField>>>,

    pub status: Option<String>,
    pub summary: Option<Vec<String>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Override parameters using the provided value if it exists.
    fn merge<T: Into<Self>>(&mut self, other: T) {
        let other = other.into();
        or!(self.assignee, other.assignee);
        or!(self.attachments, other.attachments);
        or!(self.blocks, other.blocks);
        or!(self.blocked, other.blocked);
        or!(self.relates, other.relates);
        or!(self.ids, other.ids);
        or!(self.created, other.created);
        or!(self.updated, other.updated);
        or!(self.closed, other.closed);
        or!(self.limit, other.limit);
        or!(self.offset, other.offset);
        or!(self.order, other.order);
        or!(self.status, other.status);
        or!(self.summary, other.summary);
    }
}

struct QueryBuilder<'a> {
    _service: &'a Service,
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
    fn new(_service: &'a Service) -> Self {
        Self {
            _service,
            query: Default::default(),
        }
    }

    /// Match conditionally existent array field values.
    fn exists(&mut self, field: ExistsField, status: bool) {
        let status = if status { "*" } else { "!*" };
        self.insert(field, status);
    }

    fn id(&mut self, values: &[RangeOrValue<u64>]) -> crate::Result<()> {
        let (ids, ranges): (Vec<_>, Vec<_>) = values
            .iter()
            .partition(|x| matches!(x, RangeOrValue::Value(_)));

        if !ids.is_empty() && !ranges.is_empty() {
            return Err(Error::InvalidValue(
                "IDs and ID ranges specified".to_string(),
            ));
        }

        if !ids.is_empty() {
            self.insert("issue_id", ids.iter().join(","));
        }

        match &ranges[..] {
            [] => (),
            [value] => match value {
                RangeOrValue::RangeOp(value) => self.range_op("issue_id", value),
                RangeOrValue::Range(value) => self.range("issue_id", value),
                RangeOrValue::Value(_) => (),
            },
            _ => {
                return Err(Error::InvalidValue(
                    "multiple ID ranges specified".to_string(),
                ))
            }
        }

        Ok(())
    }

    fn status(&mut self, value: &str) -> crate::Result<()> {
        match value {
            "@open" => self.append("status_id", "open"),
            "@closed" => self.append("status_id", "closed"),
            "@any" => self.append("status_id", "*"),
            // TODO: use service cache to support custom values mapped to IDs
            _ => return Err(Error::InvalidValue(format!("invalid status: {value}"))),
        }

        Ok(())
    }

    fn time(&mut self, field: &str, value: &RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => {
                let value = value.api();
                self.insert(field, format!(">={value}"));
            }
            RangeOrValue::RangeOp(value) => self.range_op(field, value),
            RangeOrValue::Range(value) => self.range(field, value),
        }
    }

    // Redmine doesn't support native < or > operators so use <= and >= for them.
    fn range_op<T>(&mut self, field: &str, value: &RangeOp<T>)
    where
        T: Api + Eq,
    {
        match value {
            RangeOp::Less(value) | RangeOp::LessOrEqual(value) => {
                let value = value.api();
                self.insert(field, format!("<={value}"));
            }
            RangeOp::Equal(value) => {
                let value = value.api();
                self.insert(field, format!("={value}"));
            }
            RangeOp::NotEqual(value) => {
                let value = value.api();
                self.insert(field, format!("!{value}"));
            }
            RangeOp::GreaterOrEqual(value) | RangeOp::Greater(value) => {
                let value = value.api();
                self.insert(field, format!(">={value}"));
            }
        }
    }

    fn range<T>(&mut self, field: &str, value: &Range<T>)
    where
        T: Api + Eq,
    {
        match value {
            Range::Range(r) => {
                let (start, end) = (r.start.api(), r.end.api());
                self.insert(field, format!("><{start}|{end}"));
            }
            Range::Inclusive(r) => {
                let (start, end) = (r.start().api(), r.end().api());
                self.insert(field, format!("><{start}|{end}"));
            }
            Range::To(r) => {
                let end = r.end.api();
                self.insert(field, format!("<={end}"));
            }
            Range::ToInclusive(r) => {
                let end = r.end.api();
                self.insert(field, format!("<={end}"));
            }
            Range::From(r) => {
                let start = r.start.api();
                self.insert(field, format!(">={start}"));
            }
            Range::Full(_) => (),
        }
    }
}

/// Quote terms containing whitespace, combining them into a query value.
fn quoted_strings<I, S>(values: I) -> String
where
    I: IntoIterator<Item = S>,
    S: fmt::Display,
{
    values
        .into_iter()
        .map(|s| {
            let s = s.to_string();
            if s.contains(char::is_whitespace) {
                format!("\"{s}\"")
            } else {
                s
            }
        })
        .join(" ")
}

#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum ExistsField {
    Assignee,
    Attachment,
    Blocks,
    Blocked,
    Relates,
}

impl Api for ExistsField {
    fn api(&self) -> String {
        let value = match self {
            Self::Assignee => "assigned_to_id",
            Self::Attachment => "attachment",
            Self::Blocks => "blocks",
            Self::Blocked => "blocked",
            Self::Relates => "relates",
        };
        value.to_string()
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, PartialEq, Eq, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum OrderField {
    /// person the issue is assigned to
    Assignee,
    /// person who created the issue
    Author,
    /// time when the issue was closed
    Closed,
    /// time when the issue was created
    Created,
    /// issue ID
    Id,
    /// issue priority
    Priority,
    /// issue status
    Status,
    /// issue subject
    Subject,
    /// issue type
    Tracker,
    /// time when the issue was last updated
    Updated,
}

impl Api for OrderField {
    fn api(&self) -> String {
        let value = match self {
            Self::Assignee => "assigned_to",
            Self::Author => "author",
            Self::Closed => "closed_on",
            Self::Created => "created_on",
            Self::Id => "id",
            Self::Priority => "priority",
            Self::Status => "status",
            Self::Subject => "subject",
            Self::Tracker => "tracker",
            Self::Updated => "updated_on",
        };
        value.to_string()
    }
}

impl Api for Order<OrderField> {
    fn api(&self) -> String {
        match self {
            Order::Ascending(field) => format!("{}:asc", field.api()),
            Order::Descending(field) => format!("{}:desc", field.api()),
        }
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::service::redmine::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("redmine");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        server
            .respond(200, path.join("search/nonexistent.json"))
            .await;

        // order
        for field in OrderField::iter() {
            service
                .search()
                .order([Order::Ascending(field)])
                .send()
                .await
                .unwrap();
        }
    }
}
