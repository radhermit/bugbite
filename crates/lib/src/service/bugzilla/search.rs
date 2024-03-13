use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::offset::Utc;
use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::objects::bugzilla::Bug;
use crate::time::TimeDelta;
use crate::traits::{Api, Query, Request, WebService};
use crate::Error;

use super::{BugField, FilterField};

// default fields to return for searches
static DEFAULT_SEARCH_FIELDS: &[BugField] = &[BugField::Id, BugField::Summary];

#[derive(Debug)]
pub(crate) struct SearchRequest(url::Url);

impl Request for SearchRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.0);
        let response = service.send(request).await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        Ok(serde_json::from_value(data)?)
    }
}

impl SearchRequest {
    pub(super) fn new<Q: Query>(service: &super::Service, mut query: Q) -> crate::Result<Self> {
        let url = service
            .base()
            .join(&format!("rest/bug?{}", query.params()?))?;
        Ok(Self(url))
    }
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Default)]
pub struct QueryBuilder {
    query: ListOrderedMultimap<String, String>,
    advanced_count: u64,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        self.extend("id", values);
    }

    pub fn comment<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "longdesc");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn summary<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "short_desc");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn created_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = datetime.format("%Y-%m-%dT%H:%M:%SZ");
        self.insert("creation_time", target);
    }

    pub fn modified_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = datetime.format("%Y-%m-%dT%H:%M:%SZ");
        self.insert("last_change_time", target);
    }

    pub fn order<'a, I>(&mut self, terms: I)
    where
        I: IntoIterator<Item = &'a SearchOrder>,
    {
        let order = terms.into_iter().map(|x| x.api()).join(",");
        self.insert("order", order);
    }

    pub fn limit(&mut self, value: NonZeroU64) {
        self.insert("limit", value);
    }

    pub fn commenter<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "commenter");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn url<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "bug_file_loc");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn votes(&mut self, value: u32) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "votes");
        self.insert(format!("o{num}"), "greaterthaneq");
        self.insert(format!("v{num}"), value);
    }

    pub fn comments(&mut self, value: u32) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "longdescs.count");
        self.insert(format!("o{num}"), "greaterthaneq");
        self.insert(format!("v{num}"), value);
    }

    pub fn attachments(&mut self, value: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "attach_data.thedata");
        if value {
            self.insert(format!("o{num}"), "isnotempty");
        } else {
            self.insert(format!("o{num}"), "isempty");
        }
    }

    pub fn groups<S>(&mut self, values: &[S])
    where
        S: fmt::Display,
    {
        if values.is_empty() {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "bug_group");
            self.insert(format!("o{num}"), "isempty");
        } else {
            for value in values {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), "bug_group");
                self.insert(format!("o{num}"), "substring");
                self.insert(format!("v{num}"), value);
            }
        }
    }

    pub fn keywords<S>(&mut self, values: &[S])
    where
        S: fmt::Display,
    {
        if values.is_empty() {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "keywords");
            self.insert(format!("o{num}"), "isempty");
        } else {
            for value in values {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), "keywords");
                self.insert(format!("o{num}"), "substring");
                self.insert(format!("v{num}"), value);
            }
        }
    }

    pub fn cc<S>(&mut self, values: &[S])
    where
        S: fmt::Display,
    {
        if values.is_empty() {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "cc");
            self.insert(format!("o{num}"), "isempty");
        } else {
            for value in values {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), "cc");
                self.insert(format!("o{num}"), "substring");
                self.insert(format!("v{num}"), value);
            }
        }
    }

    pub fn fields<I, F>(&mut self, fields: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = F>,
        F: Into<FilterField>,
    {
        let mut fields: IndexSet<_> = fields.into_iter().map(Into::into).collect();
        if fields.is_empty() {
            return Err(Error::InvalidValue("fields cannot be empty".to_string()));
        }

        // always include bug IDs in field requests
        fields.insert(FilterField::Bug(BugField::Id));

        self.insert("include_fields", fields.iter().map(|f| f.api()).join(","));
        Ok(())
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

impl Query for QueryBuilder {
    fn is_empty(&self) -> bool {
        // TODO: move the keys to skip into a trait attribute
        !self
            .query
            .keys()
            .any(|k| k != "order" && k != "include_fields")
    }

    fn params(&mut self) -> crate::Result<String> {
        if self.is_empty() {
            return Err(Error::EmptyParams);
        }

        // TODO: Move this parameter to the service struct since it's configurable on the server
        // and can be queried for the supported values.
        // only return open bugs by default
        if !self.query.contains_key("status") {
            for value in ["UNCONFIRMED", "CONFIRMED", "IN_PROGRESS"] {
                self.append("status", value);
            }
        }

        // limit requested fields by default to decrease bandwidth and speed up response
        if !self.query.contains_key("include_fields") {
            self.insert(
                "include_fields",
                DEFAULT_SEARCH_FIELDS.iter().map(|f| f.api()).join(","),
            );
        }

        let mut params = url::form_urlencoded::Serializer::new(String::new());
        params.extend_pairs(self.query.iter());
        Ok(params.finish())
    }
}

#[derive(Debug, Clone, Copy)]
enum OrderType {
    Ascending,
    Descending,
}

/// Invertable search order sorting term.
#[derive(Debug, Clone, Copy)]
pub struct SearchOrder {
    order: OrderType,
    term: SearchTerm,
}

impl FromStr for SearchOrder {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let (order, term) = if let Some(value) = s.strip_prefix('-') {
            (OrderType::Descending, value)
        } else {
            (OrderType::Ascending, s.strip_prefix('+').unwrap_or(s))
        };
        let term = term
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown search term: {term}")))?;
        Ok(Self { order, term })
    }
}

impl fmt::Display for SearchOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.term.api();
        match self.order {
            OrderType::Descending => write!(f, "-{name}"),
            OrderType::Ascending => write!(f, "{name}"),
        }
    }
}

impl Api for SearchOrder {
    type Output = String;
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> Self::Output {
        let name = self.term.api();
        match self.order {
            OrderType::Descending => format!("{name} DESC"),
            OrderType::Ascending => format!("{name} ASC"),
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum SearchTerm {
    Alias,
    AssignedTo,
    Blocks,
    Comments,
    Component,
    Created,
    Deadline,
    DependsOn,
    Id,
    Keywords,
    LastVisit,
    Modified,
    Os,
    Platform,
    Priority,
    Product,
    Reporter,
    Resolution,
    Severity,
    Status,
    Summary,
    Target,
    Version,
    Votes,
}

impl Api for SearchTerm {
    type Output = &'static str;
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> Self::Output {
        match self {
            Self::Alias => "alias",
            Self::AssignedTo => "assigned_to",
            Self::Blocks => "blocked",
            Self::Comments => "longdescs.count",
            Self::Component => "component",
            Self::Created => "opendate",
            Self::Deadline => "deadline",
            Self::DependsOn => "dependson",
            Self::Id => "bug_id",
            Self::Keywords => "keywords",
            Self::LastVisit => "last_visit_ts",
            Self::Modified => "changeddate",
            Self::Os => "op_sys",
            Self::Platform => "platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Reporter => "reporter",
            Self::Resolution => "resolution",
            Self::Severity => "bug_severity",
            Self::Status => "bug_status",
            Self::Summary => "short_desc",
            Self::Target => "target_milestone",
            Self::Version => "version",
            Self::Votes => "votes",
        }
    }
}
