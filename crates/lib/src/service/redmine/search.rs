use std::collections::HashMap;
use std::fmt;

use itertools::Itertools;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::objects::redmine::Issue;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, OrderType};
use crate::time::TimeDelta;
use crate::traits::{Api, InjectAuth, Query, Request, ServiceParams, WebService};
use crate::Error;

/// Construct a search query.
#[derive(Debug)]
pub struct QueryBuilder<'a> {
    _service: &'a super::Service,
    query: HashMap<String, String>,
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

impl QueryBuilder<'_> {
    pub fn assignee(&mut self, value: bool) {
        self.exists(ExistsField::Assignee, value)
    }

    pub fn attachments<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        let value = quoted_strings(values);
        // TODO: support other operators, currently this specifies the `contains` op
        self.insert("attachment", format!("~{value}"));
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        let value = values.into_iter().join(",");
        self.insert("blocks", value);
    }

    pub fn blocked<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        let value = values.into_iter().join(",");
        self.insert("blocked", value);
    }

    pub fn relates<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        let value = values.into_iter().join(",");
        self.insert("relates", value);
    }

    pub fn id<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        let value = values.into_iter().join(",");
        self.insert("issue_id", value);
    }

    pub fn limit(&mut self, value: u64) {
        self.insert("limit", value);
    }

    pub fn order<I, T>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: TryInto<Order<OrderField>>,
        <T as TryInto<Order<OrderField>>>::Error: std::fmt::Display,
    {
        let values: Vec<_> = values
            .into_iter()
            .map(|x| x.try_into())
            .try_collect()
            .map_err(|e| Error::InvalidValue(format!("{e}")))?;
        let value = values.iter().map(|x| x.api()).join(",");
        self.insert("sort", value);
        Ok(())
    }

    pub fn status(&mut self, value: &str) -> crate::Result<()> {
        // TODO: move valid status search values to an enum
        match value {
            "open" | "@open" => self.insert("status_id", "open"),
            "closed" | "@closed" => self.insert("status_id", "closed"),
            "all" | "@all" => self.insert("status_id", "*"),
            _ => return Err(Error::InvalidValue(format!("invalid status: {value}"))),
        }
        Ok(())
    }

    pub fn closed(&mut self, value: &RangeOrValue<TimeDelta>) {
        match value {
            RangeOrValue::Value(value) => {
                self.insert("closed_on", format!(">={value}"));
            }
            RangeOrValue::RangeOp(value) => self.range_op("closed_on", value),
            RangeOrValue::Range(value) => self.range("closed_on", value),
        }
    }

    pub fn created(&mut self, value: &RangeOrValue<TimeDelta>) {
        match value {
            RangeOrValue::Value(value) => {
                self.insert("created_on", format!(">={value}"));
            }
            RangeOrValue::RangeOp(value) => self.range_op("created_on", value),
            RangeOrValue::Range(value) => self.range("created_on", value),
        }
    }

    pub fn modified(&mut self, value: &RangeOrValue<TimeDelta>) {
        match value {
            RangeOrValue::Value(value) => self.insert("updated_on", format!(">={value}")),
            RangeOrValue::RangeOp(value) => self.range_op("updated_on", value),
            RangeOrValue::Range(value) => self.range("updated_on", value),
        }
    }

    pub fn summary<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        let value = quoted_strings(values);
        // TODO: support other operators, currently this specifies the `contains` op
        self.insert("subject", format!("~{value}"));
    }

    /// Match conditionally existent array field values.
    pub fn exists(&mut self, field: ExistsField, status: bool) {
        let status = if status { "*" } else { "!*" };
        self.insert(field.api(), status);
    }

    // Redmine doesn't support native < or > operators so use <= and >= for them.
    fn range_op<T>(&mut self, field: &str, value: &RangeOp<T>)
    where
        T: fmt::Display,
    {
        match value {
            RangeOp::Less(value) | RangeOp::LessOrEqual(value) => {
                self.insert(field, format!("<={value}"));
            }
            RangeOp::Equal(value) => {
                self.insert(field, format!("={value}"));
            }
            RangeOp::NotEqual(value) => {
                self.insert(field, format!("!{value}"));
            }
            RangeOp::GreaterOrEqual(value) | RangeOp::Greater(value) => {
                self.insert(field, format!(">={value}"));
            }
        }
    }

    fn range<T>(&mut self, field: &str, value: &Range<T>)
    where
        T: fmt::Display,
    {
        match value {
            Range::Range(r) => {
                self.insert(field, format!("><{}|{}", r.start, r.end));
            }
            Range::Inclusive(r) => {
                self.insert(field, format!("><{}|{}", r.start(), r.end()));
            }
            Range::To(r) => {
                self.insert(field, format!("<={}", r.end));
            }
            Range::ToInclusive(r) => {
                self.insert(field, format!("<={}", r.end));
            }
            Range::From(r) => {
                self.insert(field, format!(">={}", r.start));
            }
            Range::Full(_) => (),
        }
    }

    fn insert<K, V>(&mut self, key: K, value: V)
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
            self.status("open")?;
        }

        // default to the common maximum limit, without this the default limit is used
        if !self.query.contains_key("limit") {
            self.limit(100);
        }

        // sort by ascending ID by default
        if !self.query.contains_key("sort") {
            self.order(["+id"])?;
        }

        params.extend_pairs(self.query.iter());
        Ok(params.finish())
    }
}

#[derive(Debug)]
pub(crate) struct SearchRequest<'a> {
    url: url::Url,
    service: &'a super::Service,
}

impl<'a> SearchRequest<'a> {
    pub(super) fn new<Q: Query>(service: &'a super::Service, mut query: Q) -> crate::Result<Self> {
        let url = service
            .base()
            .join(&format!("issues.json?{}", query.params()?))?;
        Ok(Self { url, service })
    }
}

impl Request for SearchRequest<'_> {
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url)
            .inject_auth(self.service, false)?;
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
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum OrderField {
    Assignee,
    Closed,
    Created,
    Id,
    Priority,
    Status,
    Subject,
    Tracker,
    Updated,
}

impl Api for OrderField {
    fn api(&self) -> String {
        let value = match self {
            Self::Assignee => "assigned_to",
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
