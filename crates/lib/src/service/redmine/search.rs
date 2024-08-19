use std::ops::{Deref, DerefMut};
use std::{fmt, fs};

use camino::Utf8Path;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::args::ExistsOrValues;
use crate::objects::redmine::Issue;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{self, Order, OrderType};
use crate::time::TimeDeltaOrStatic;
use crate::traits::{Api, InjectAuth, RequestSend, WebService};
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

    /// Match conditionally existent array field values.
    fn exists(&mut self, field: ExistsField, status: bool) {
        let status = if status { "*" } else { "!*" };
        self.insert(field.api(), status);
    }

    fn id<I>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = RangeOrValue<u64>>,
    {
        let (ids, ranges): (Vec<_>, Vec<_>) = values
            .into_iter()
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

    fn time(&mut self, field: &str, value: RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => {
                let value = value.api();
                self.insert(field, format!(">={value}"));
            }
            RangeOrValue::RangeOp(value) => self.range_op(field, &value),
            RangeOrValue::Range(value) => self.range(field, &value),
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

    pub limit: Option<u64>,
    pub order: Option<Vec<Order<OrderField>>>,
    pub status: Option<String>,
    pub summary: Option<Vec<String>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    pub fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Merge parameters using the provided value for fallbacks.
    pub fn merge<T: Into<Self>>(&mut self, other: T) {
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
        or!(self.order, other.order);
        or!(self.status, other.status);
        or!(self.summary, other.summary);
    }

    pub fn order<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = Order<OrderField>>,
    {
        self.order = Some(values.into_iter().collect());
        self
    }

    pub fn status<S: AsRef<str>>(mut self, value: S) -> crate::Result<Self> {
        // TODO: move valid status search values to an enum
        match value.as_ref() {
            "open" => self.status = Some("open".to_string()),
            "closed" => self.status = Some("closed".to_string()),
            "all" => self.status = Some("*".to_string()),
            x => return Err(Error::InvalidValue(format!("invalid status: {x}"))),
        }
        Ok(self)
    }

    pub(crate) fn encode(self, service: &super::Service) -> crate::Result<String> {
        let mut query = QueryBuilder::new(service);

        if let Some(values) = self.blocks {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocks, value),
                ExistsOrValues::Values(values) => query.insert("blocks", values.iter().join(",")),
            }
        }

        if let Some(values) = self.blocked {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocked, value),
                ExistsOrValues::Values(values) => query.insert("blocked", values.iter().join(",")),
            }
        }

        if let Some(values) = self.relates {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Relates, value),
                ExistsOrValues::Values(values) => query.insert("relates", values.iter().join(",")),
            }
        }

        if let Some(values) = self.ids {
            query.id(values)?;
        }

        if let Some(value) = self.closed {
            query.time("closed_on", value);
        }

        if let Some(value) = self.created {
            query.time("created_on", value);
        }

        if let Some(value) = self.updated {
            query.time("updated_on", value);
        }

        if let Some(value) = self.assignee {
            query.exists(ExistsField::Assignee, value);
        }

        if let Some(values) = self.attachments {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Attachment, value),
                ExistsOrValues::Values(values) => {
                    let value = quoted_strings(values);
                    // TODO: support other operators, currently this specifies the `contains` op
                    query.insert("attachment", format!("~{value}"));
                }
            }
        }

        if let Some(values) = self.summary {
            let value = quoted_strings(values);
            // TODO: support other operators, currently this specifies the `contains` op
            query.insert("subject", format!("~{value}"));
        }

        if let Some(value) = self.status {
            query.insert("status", value);
        } else {
            // limit to open issues by default
            query.insert("status", "open");
        }

        if let Some(values) = self.order {
            let value = values.iter().map(|x| x.api()).join(",");
            query.insert("sort", value);
        } else {
            // sort by ascending ID by default
            let order = Order::ascending(OrderField::Id);
            query.insert("sort", order.api());
        }

        if let Some(value) = self.limit {
            query.insert("limit", value);
        } else {
            // default to the common maximum limit, without this the default limit is used
            query.insert("limit", "100");
        }

        Ok(query.encode())
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

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a super::Service,
    params: Parameters,
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a super::Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    pub fn params(mut self, params: Parameters) -> Self {
        self.params = params;
        self
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        let params = self.params.encode(self.service)?;
        let url = self
            .service
            .config
            .base
            .join(&format!("issues.json?{params}"))?;
        let request = self.service.client.get(url).auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["issues"].take();
        let issues = serde_json::from_value(data)
            .map_err(|e| Error::InvalidValue(format!("failed deserializing issues: {e}")))?;
        Ok(issues)
    }
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
        let name = self.field.api();
        match self.order {
            OrderType::Descending => format!("{name}:desc"),
            OrderType::Ascending => format!("{name}:asc"),
        }
    }
}
