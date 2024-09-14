use std::fmt;
use std::ops::{Deref, DerefMut};

use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString};
use url::Url;

use crate::args::ExistsOrValues;
use crate::objects::redmine::Issue;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, Query};
use crate::service::redmine::Redmine;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{
    Api, InjectAuth, Merge, MergeOption, RequestStream, RequestTemplate, WebService,
};
use crate::Error;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request {
    #[serde(skip)]
    service: Redmine,
    #[serde(flatten)]
    pub params: Parameters,
}

/// Iterator of consecutive, paged requests.
struct PagedIterator {
    paged: usize,
    request: Request,
}

impl Iterator for PagedIterator {
    type Item = Request;

    fn next(&mut self) -> Option<Self::Item> {
        let req = self.request.clone();
        self.request.params.offset = self
            .request
            .params
            .offset
            .unwrap_or_default()
            .checked_add(self.paged);
        req.params.offset.map(|_| req)
    }
}

impl RequestStream for Request {
    type Item = Issue;

    fn paged(&mut self) -> Option<usize> {
        if self.params.paged.unwrap_or_default() || self.params.limit.is_none() {
            self.params
                .limit
                .get_or_insert_with(|| self.service.config.max_search_results());
            self.params.offset.get_or_insert_with(Default::default);
            self.params.limit
        } else {
            None
        }
    }

    fn paged_requests(self, paged: Option<usize>) -> impl Iterator<Item = Self> {
        if let Some(value) = paged {
            Either::Left(PagedIterator {
                paged: value,
                request: self,
            })
        } else {
            Either::Right([self].into_iter())
        }
    }

    async fn send(self) -> crate::Result<Vec<Issue>> {
        let mut url = self.service.config.base.join("issues.json")?;
        let query = self.encode()?;
        url.query_pairs_mut().extend_pairs(query.iter());
        let request = self.service.client.get(url).auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["issues"].take();
        serde_json::from_value(data)
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing issues: {e}")))
    }
}

impl Request {
    pub(super) fn new(service: &Redmine) -> Self {
        Self {
            service: service.clone(),
            params: Default::default(),
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(&self.service);

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

        if let Some(values) = &self.params.subject {
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
        let query = self.encode()?;
        url.query_pairs_mut().extend_pairs(query.iter());
        Ok(url)
    }

    pub fn id<T>(mut self, value: T) -> Self
    where
        T: Into<RangeOrValue<u64>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .ids
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn created(mut self, value: RangeOrValue<TimeDeltaOrStatic>) -> Self {
        self.params.created = Some(value);
        self
    }

    pub fn updated(mut self, value: RangeOrValue<TimeDeltaOrStatic>) -> Self {
        self.params.updated = Some(value);
        self
    }

    pub fn closed(mut self, value: RangeOrValue<TimeDeltaOrStatic>) -> Self {
        self.params.closed = Some(value);
        self
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

    pub fn subject<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.subject = Some(values.into_iter().map(Into::into).collect());
        self
    }
}

impl RequestTemplate for Request {
    type Params = Parameters;
    type Service = Redmine;
    const TYPE: &'static str = "search";

    fn service(&self) -> &Self::Service {
        &self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

/// Issue search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
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
    pub paged: Option<bool>,

    pub status: Option<String>,
    pub subject: Option<Vec<String>>,
}

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            assignee: self.assignee.merge(other.assignee),
            attachments: self.attachments.merge(other.attachments),
            blocks: self.blocks.merge(other.blocks),
            blocked: self.blocked.merge(other.blocked),
            relates: self.relates.merge(other.relates),
            ids: self.ids.merge(other.ids),
            created: self.created.merge(other.created),
            updated: self.updated.merge(other.updated),
            closed: self.closed.merge(other.closed),
            limit: self.limit.merge(other.limit),
            offset: self.offset.merge(other.offset),
            order: self.order.merge(other.order),
            paged: self.paged.merge(other.paged),
            status: self.status.merge(other.status),
            subject: self.subject.merge(other.subject),
        }
    }
}

struct QueryBuilder<'a> {
    _service: &'a Redmine,
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
    fn new(_service: &'a Redmine) -> Self {
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

        if !ids.is_empty() {
            if !ranges.is_empty() {
                return Err(Error::InvalidValue(
                    "IDs and ID ranges specified".to_string(),
                ));
            }

            self.insert("issue_id", ids.iter().join(","));
        }

        if ranges.len() > 1 {
            return Err(Error::InvalidValue(
                "multiple ID ranges specified".to_string(),
            ));
        } else if let Some(value) = ranges.first() {
            match value {
                RangeOrValue::RangeOp(value) => self.range_op("issue_id", value),
                RangeOrValue::Range(value) => self.range("issue_id", value),
                RangeOrValue::Value(_) => unreachable!("failed partitioning values"),
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

#[derive(Display, EnumIter, EnumString, Debug, Clone, Copy)]
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
#[derive(Display, EnumIter, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
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

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("redmine");
        let server = TestServer::new().await;
        let service = Redmine::new(server.uri()).unwrap();

        server
            .respond(200, path.join("search/nonexistent.json"))
            .await;

        // valid operator-based ID ranges
        let id_ranges = ["<10", "<=10", "=10", "!=10", ">=10", ">10"];

        // valid TimeDeltaOrStatic values
        let times = vec![
            "2020",
            "2020-02",
            "2020-02-01",
            "2020-02-01T01:02:03Z",
            "1h",
            "<1d",
            "<=1w",
            ">=1m",
            ">1y",
            "2020..2021",
            "2020..=2021",
            "..2021",
            "..=2021",
            "2021..",
            "..",
        ];

        // ids
        service.search().id(1).send().await.unwrap();
        service.search().id(10..20).send().await.unwrap();
        service.search().id(10..=20).send().await.unwrap();
        service.search().id(..20).send().await.unwrap();
        service.search().id(..=20).send().await.unwrap();
        service.search().id(10..).send().await.unwrap();
        service.search().id(..).send().await.unwrap();
        for s in &id_ranges {
            let range: RangeOrValue<u64> = s.parse().unwrap();
            service.search().id(range).send().await.unwrap();
        }
        let err = service.search().id(10).id(10..).send().await.unwrap_err();
        assert_err_re!(err, "IDs and ID ranges specified");
        let err = service.search().id(..10).id(10..).send().await.unwrap_err();
        assert_err_re!(err, "multiple ID ranges specified");

        // time related combinators
        for time in &times {
            // created
            service
                .search()
                .created(time.parse().unwrap())
                .send()
                .await
                .unwrap();

            // updated
            service
                .search()
                .updated(time.parse().unwrap())
                .send()
                .await
                .unwrap();

            // closed
            service
                .search()
                .closed(time.parse().unwrap())
                .send()
                .await
                .unwrap();
        }

        // order
        for field in OrderField::iter() {
            service
                .search()
                .order([Order::Ascending(field)])
                .send()
                .await
                .unwrap();
        }

        // subject
        service.search().subject(["test"]).send().await.unwrap();
        service
            .search()
            .subject(["test1", "test2"])
            .send()
            .await
            .unwrap();
        service
            .search()
            .subject(["test with whitespace"])
            .send()
            .await
            .unwrap();
    }
}
