use std::fmt;
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

/// Advanced field matching operators.
#[derive(Display, Debug, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
enum MatchOp {
    Substring,
    NotSubstring,
    Equals,
    NotEquals,
    Regexp,
    NotRegexp,
}

/// Advanced field match.
#[derive(Debug, Clone)]
pub struct Match {
    op: MatchOp,
    value: String,
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl FromStr for Match {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.into())
    }
}

impl From<&str> for Match {
    fn from(s: &str) -> Self {
        let (op, value) = match s.split_once('#') {
            Some(("!", value)) => (MatchOp::NotSubstring, value.into()),
            Some(("=", value)) => (MatchOp::Equals, value.into()),
            Some(("!=", value)) => (MatchOp::NotEquals, value.into()),
            Some(("r", value)) => (MatchOp::Regexp, value.into()),
            Some(("!r", value)) => (MatchOp::NotRegexp, value.into()),
            _ => (MatchOp::Substring, s.into()),
        };

        Self { op, value }
    }
}

impl From<String> for Match {
    fn from(s: String) -> Self {
        s.as_str().into()
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
        I: IntoIterator<Item = u64>,
    {
        self.extend("id", values);
    }

    pub fn alias<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.or("alias", values)
    }

    pub fn assigned_to<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.or("assigned_to", values);
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
            self.advanced_field("attachments.description", value.op, &value);
            self.advanced_field("attachments.filename", value.op, &value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    pub fn reporter<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.or("reporter", values);
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
        self.and("longdesc", values)
    }

    pub fn summary<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.and("short_desc", values)
    }

    pub fn created_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = datetime.format("%Y-%m-%dT%H:%M:%SZ");
        self.advanced_field("creation_ts", "greaterthaneq", target);
    }

    pub fn modified_after(&mut self, interval: &TimeDelta) {
        let datetime = Utc::now() - interval.delta();
        let target = datetime.format("%Y-%m-%dT%H:%M:%SZ");
        self.advanced_field("delta_ts", "greaterthaneq", target);
    }

    pub fn order<I, T>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: TryInto<Order>,
        <T as TryInto<Order>>::Error: std::fmt::Display,
    {
        let values = values
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::InvalidValue(format!("{e}")))?;
        let value = values.into_iter().map(|x| x.api()).join(",");
        self.insert("order", value);
        Ok(())
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
        self.and("attachments.submitter", values)
    }

    pub fn commenters<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.and("commenter", values)
    }

    pub fn url<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_field("bug_file_loc", "substring", value);
        }
    }

    pub fn changed_by<I, K, V>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = (K, V)>,
        K: TryInto<ChangeField> + fmt::Display + Copy,
        <K as TryInto<ChangeField>>::Error: std::fmt::Display,
        V: fmt::Display,
    {
        for (field, value) in values {
            let field = field
                .try_into()
                .map_err(|_| Error::InvalidValue(format!("invalid change field: {field}")))?;
            self.advanced_field(field.api(), "changedby", value);
        }
        Ok(())
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
            let value = value.into();
            self.advanced_field(name, value.op, value);
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
                "@open" => self.append("bug_status", "__open__"),
                "@closed" => self.append("bug_status", "__closed__"),
                "@all" => self.append("bug_status", "__all__"),
                s => self.append("bug_status", s),
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
            self.advanced_field("see_also", "substring", value);
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
        I: IntoIterator<Item = u64>,
    {
        for value in values {
            self.advanced_field("blocked", "equals", value);
        }
    }

    pub fn depends_on<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = u64>,
    {
        for value in values {
            self.advanced_field("dependson", "equals", value);
        }
    }

    pub fn flags<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        for value in values {
            self.advanced_field("flagtypes.name", "substring", value);
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
        S: Into<Match>,
    {
        self.and("keywords", values)
    }

    pub fn cc<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.and("cc", values)
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
            RangeOrEqual::Equal(value) => self.advanced_field(field, "equals", value),
            RangeOrEqual::Range(Range::Between(start, finish)) => {
                self.advanced_field(field, "greaterthaneq", start);
                self.advanced_field(field, "lessthan", finish);
            }
            RangeOrEqual::Range(Range::Inclusive(start, finish)) => {
                self.advanced_field(field, "greaterthaneq", start);
                self.advanced_field(field, "lessthaneq", finish);
            }
            RangeOrEqual::Range(Range::To(value)) => {
                self.advanced_field(field, "lessthan", value);
            }
            RangeOrEqual::Range(Range::ToInclusive(value)) => {
                self.advanced_field(field, "lessthaneq", value);
            }
            RangeOrEqual::Range(Range::From(value)) => {
                self.advanced_field(field, "greaterthaneq", value);
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

    fn advanced_field<F, K, V>(&mut self, field: F, operator: K, value: V)
    where
        F: fmt::Display,
        K: fmt::Display,
        V: fmt::Display,
    {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), field);
        self.insert(format!("o{num}"), operator);
        self.insert(format!("v{num}"), value);
    }

    fn op<F, I, S>(&mut self, op: &str, field: F, values: I)
    where
        F: fmt::Display + Copy,
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), op);

        for value in values.into_iter().map(Into::into) {
            self.advanced_field(field, value.op, value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    fn and<F, I, S>(&mut self, field: F, values: I)
    where
        F: fmt::Display + Copy,
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op("AND", field, values)
    }

    fn or<F, I, S>(&mut self, field: F, values: I)
    where
        F: fmt::Display + Copy,
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op("OR", field, values)
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
        if !self.query.contains_key("bug_status") {
            self.status(["@open"]);
        }

        // sort by ascending ID by default
        if !self.query.contains_key("order") {
            self.order(["id"])?;
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
    Flags,
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
            Self::Attachments => "attachments.submitter",
            Self::Blocks => "blocked",
            Self::Cc => "cc",
            Self::DependsOn => "dependson",
            Self::Flags => "setters.login_name",
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
pub struct Order {
    order: OrderType,
    field: OrderField,
}

impl TryFrom<&str> for Order {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for Order {
    type Err = Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        let (order, field) = if let Some(value) = s.strip_prefix('-') {
            (OrderType::Descending, value)
        } else {
            (OrderType::Ascending, s.strip_prefix('+').unwrap_or(s))
        };
        let field = field
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown search field: {field}")))?;
        Ok(Self { order, field })
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = self.field.api();
        match self.order {
            OrderType::Descending => write!(f, "-{name}"),
            OrderType::Ascending => write!(f, "{name}"),
        }
    }
}

impl Api for Order {
    type Output = String;
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> Self::Output {
        let name = self.field.api();
        match self.order {
            OrderType::Descending => format!("{name} DESC"),
            OrderType::Ascending => format!("{name} ASC"),
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum OrderField {
    Alias,
    AssignedTo,
    Blocks,
    Comments,
    Component,
    Created,
    Deadline,
    DependsOn,
    Flags,
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
    Whiteboard,
}

impl Api for OrderField {
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
            Self::Flags => "flagtypes.name",
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
            Self::Whiteboard => "status_whiteboard",
        }
    }
}

/// Valid change fields.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum ChangeField {
    Alias,
    AssignedTo,
    Blocks,
    Component,
    Cc,
    Deadline,
    DependsOn,
    Flags,
    Keywords,
    Os,
    Platform,
    Priority,
    Product,
    Reporter,
    Resolution,
    SeeAlso,
    Severity,
    Status,
    Summary,
    Target,
    Url,
    Version,
    Votes,
    Whiteboard,
}

impl Api for ChangeField {
    type Output = &'static str;
    /// Translate a search order variant into the expected REST API v1 name.
    fn api(&self) -> Self::Output {
        match self {
            Self::Alias => "alias",
            Self::AssignedTo => "assigned_to",
            Self::Blocks => "blocked",
            Self::Component => "component",
            Self::Cc => "cc",
            Self::Deadline => "deadline",
            Self::DependsOn => "dependson",
            Self::Flags => "flagtypes.name",
            Self::Keywords => "keywords",
            Self::Os => "op_sys",
            Self::Platform => "platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Reporter => "reporter",
            Self::Resolution => "resolution",
            Self::SeeAlso => "see_also",
            Self::Severity => "bug_severity",
            Self::Status => "bug_status",
            Self::Summary => "short_desc",
            Self::Target => "target_milestone",
            Self::Url => "bug_file_loc",
            Self::Version => "version",
            Self::Votes => "votes",
            Self::Whiteboard => "status_whiteboard",
        }
    }
}

impl TryFrom<String> for ChangeField {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value
            .parse()
            .map_err(|_| Error::InvalidValue(format!("unknown change field: {value}")))
    }
}
