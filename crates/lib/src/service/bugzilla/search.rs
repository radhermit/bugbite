use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::offset::Utc;
use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOrEqual};
use crate::time::TimeDelta;
use crate::traits::{Api, InjectAuth, Query, Request, ServiceParams, WebService};
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
        let request = service.client().get(self.0).inject_auth(service, false)?;
        let response = request.send().await?;
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

/// Variants for matching types.
#[derive(Debug, Clone)]
pub enum Match {
    Contains(String),
    ContainsNot(String),
    Equals(String),
    EqualsNot(String),
    Regex(String),
    RegexNot(String),
}

impl Match {
    fn op(&self) -> &str {
        match self {
            Self::Contains(_) => "substring",
            Self::ContainsNot(_) => "notsubstring",
            Self::Equals(_) => "equals",
            Self::EqualsNot(_) => "notequals",
            Self::Regex(_) => "regexp",
            Self::RegexNot(_) => "notregexp",
        }
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Contains(value) => value.fmt(f),
            Self::ContainsNot(value) => value.fmt(f),
            Self::Equals(value) => value.fmt(f),
            Self::EqualsNot(value) => value.fmt(f),
            Self::Regex(value) => value.fmt(f),
            Self::RegexNot(value) => value.fmt(f),
        }
    }
}

impl FromStr for Match {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl<T: AsRef<str>> From<T> for Match {
    fn from(s: T) -> Self {
        let s = s.as_ref();
        match s.split_once('#') {
            Some(("!", value)) => Self::ContainsNot(value.into()),
            Some(("=", value)) => Self::Equals(value.into()),
            Some(("!=", value)) => Self::EqualsNot(value.into()),
            Some(("r", value)) => Self::Regex(value.into()),
            Some(("!r", value)) => Self::RegexNot(value.into()),
            _ => Self::Contains(s.into()),
        }
    }
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug)]
pub struct QueryBuilder<'a> {
    _service: &'a super::Service,
    query: ListOrderedMultimap<String, String>,
    advanced_count: u64,
}

impl<'a> ServiceParams<'a> for QueryBuilder<'a> {
    type Service = super::Service;

    fn new(_service: &'a Self::Service) -> Self {
        Self {
            _service,
            query: Default::default(),
            advanced_count: Default::default(),
        }
    }
}

impl QueryBuilder<'_> {
    pub fn id<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        self.extend("id", values);
    }

    pub fn alias<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "alias");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn assigned_to<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("assigned_to", values);
    }

    /// Search for attachments with matching descriptions or filenames.
    pub fn attachments<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), "OR");

        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "attachments.description");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), &value);

            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "attachments.filename");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), &value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    pub fn creator<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("creator", values);
    }

    pub fn resolution<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("resolution", values);
    }

    pub fn comment<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "longdesc");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn summary<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "short_desc");
            self.insert(format!("o{num}"), value.op());
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

    pub fn order<I>(&mut self, terms: I)
    where
        I: IntoIterator<Item = SearchOrder>,
    {
        let order = terms.into_iter().map(|x| x.api()).join(",");
        self.insert("order", order);
    }

    pub fn limit(&mut self, value: u64) {
        self.insert("limit", value);
    }

    pub fn quicksearch(&mut self, value: String) {
        self.insert("quicksearch", value);
    }

    pub fn attachers<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "attachments.submitter");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn commenters<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "commenter");
            self.insert(format!("o{num}"), value.op());
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

    pub fn custom_fields<I, K, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<Match>,
    {
        for (name, value) in values {
            let name = match name.as_ref() {
                k if k.starts_with("cf_") => k.into(),
                k => format!("cf_{k}"),
            };
            self.advanced_count += 1;
            let num = self.advanced_count;
            let value = value.into();
            self.insert(format!("f{num}"), name);
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn priority<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("priority", values);
    }

    pub fn severity<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("bug_severity", values);
    }

    pub fn status<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for value in values {
            match value.as_ref() {
                "@open" => self.append("status", "__open__"),
                "@closed" => self.append("status", "__closed__"),
                "@all" => self.append("status", "__all__"),
                s => self.append("status", s),
            }
        }
    }

    pub fn version<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("version", values);
    }

    pub fn component<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("component", values);
    }

    pub fn product<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("product", values);
    }

    pub fn platform<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("platform", values);
    }

    pub fn os<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("op_sys", values);
    }

    pub fn see_also<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "see_also");
            self.insert(format!("o{num}"), "substring");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn target<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("target_milestone", values);
    }

    pub fn whiteboard<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("whiteboard", values);
    }

    pub fn votes(&mut self, value: RangeOrEqual<u64>) {
        self.range("votes", value)
    }

    pub fn comments(&mut self, value: RangeOrEqual<u64>) {
        self.range("longdescs.count", value)
    }

    /// Match bugs with conditionally existent array field values.
    pub fn exists(&mut self, field: ExistsField, status: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        let status = if status { "isnotempty" } else { "isempty" };
        self.insert(format!("f{num}"), field.api());
        self.insert(format!("o{num}"), status);
    }

    pub fn blocks<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "blocked");
            self.insert(format!("o{num}"), "equals");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn depends_on<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = NonZeroU64>,
    {
        for value in values {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "dependson");
            self.insert(format!("o{num}"), "equals");
            self.insert(format!("v{num}"), value);
        }
    }

    pub fn groups<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("bug_group", values);
    }

    pub fn keywords<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        self.extend("keywords", values);
    }

    pub fn cc<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        for value in values.into_iter().map(Into::into) {
            self.advanced_count += 1;
            let num = self.advanced_count;
            self.insert(format!("f{num}"), "cc");
            self.insert(format!("o{num}"), value.op());
            self.insert(format!("v{num}"), value);
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

    fn range<T>(&mut self, field: &str, value: RangeOrEqual<T>)
    where
        T: fmt::Display,
    {
        match value {
            RangeOrEqual::Equal(value) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "equals");
                self.insert(format!("v{num}"), value);
            }
            RangeOrEqual::Range(Range::Between(start, finish)) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "greaterthaneq");
                self.insert(format!("v{num}"), start);

                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "lessthan");
                self.insert(format!("v{num}"), finish);
            }
            RangeOrEqual::Range(Range::Inclusive(start, finish)) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "greaterthaneq");
                self.insert(format!("v{num}"), start);

                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "lessthaneq");
                self.insert(format!("v{num}"), finish);
            }
            RangeOrEqual::Range(Range::To(value)) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "lessthan");
                self.insert(format!("v{num}"), value);
            }
            RangeOrEqual::Range(Range::ToInclusive(value)) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "lessthaneq");
                self.insert(format!("v{num}"), value);
            }
            RangeOrEqual::Range(Range::From(value)) => {
                self.advanced_count += 1;
                let num = self.advanced_count;
                self.insert(format!("f{num}"), field);
                self.insert(format!("o{num}"), "greaterthaneq");
                self.insert(format!("v{num}"), value);
            }
            RangeOrEqual::Range(Range::Full) => (),
        }
    }

    fn extend<K, I, V>(&mut self, key: K, values: I)
    where
        I: IntoIterator<Item = V>,
        K: fmt::Display,
        V: fmt::Display,
    {
        for value in values {
            self.query.append(key.to_string(), value.to_string());
        }
    }

    fn append<K, V>(&mut self, key: K, value: V)
    where
        K: fmt::Display,
        V: fmt::Display,
    {
        self.query.append(key.to_string(), value.to_string());
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

        // only return open bugs by default
        if !self.query.contains_key("status") {
            self.status(["@open"]);
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

/// Bug fields composed of value arrays.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum ExistsField {
    Alias,
    Attachments,
    Blocks,
    Cc,
    DependsOn,
    Groups,
    Keywords,
    SeeAlso,
    Url,
    Whiteboard,
}

impl Api for ExistsField {
    type Output = &'static str;
    fn api(&self) -> Self::Output {
        match self {
            Self::Alias => "alias",
            Self::Attachments => "attach_data.thedata",
            Self::Blocks => "blocked",
            Self::Cc => "cc",
            Self::DependsOn => "dependson",
            Self::Groups => "bug_group",
            Self::Keywords => "keywords",
            Self::SeeAlso => "see_also",
            Self::Url => "bug_file_loc",
            Self::Whiteboard => "status_whiteboard",
        }
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
    Url,
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
            Self::Created => "creation_ts",
            Self::Deadline => "deadline",
            Self::DependsOn => "dependson",
            Self::Id => "bug_id",
            Self::Keywords => "keywords",
            Self::LastVisit => "last_visit_ts",
            Self::Modified => "delta_ts",
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
            Self::Url => "bug_file_loc",
            Self::Version => "version",
            Self::Votes => "votes",
        }
    }
}
