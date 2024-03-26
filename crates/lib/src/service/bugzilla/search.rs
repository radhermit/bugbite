use std::str::FromStr;
use std::{fmt, iter};

use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, OrderType};
use crate::time::TimeDeltaIso8601;
use crate::traits::{Api, InjectAuth, Query, Request, ServiceParams, WebService};
use crate::Error;

use super::{BugField, FilterField};

#[derive(Debug)]
pub(crate) struct SearchRequest<'a> {
    url: url::Url,
    service: &'a super::Service,
}

impl Request for SearchRequest<'_> {
    type Output = Vec<Bug>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url)
            .inject_auth(self.service, false)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let bugs = serde_json::from_value(data)
            .map_err(|e| Error::InvalidValue(format!("failed deserializing bugs: {e}")))?;
        Ok(bugs)
    }
}

impl<'a> SearchRequest<'a> {
    pub(super) fn new(service: &'a super::Service, mut query: QueryBuilder) -> crate::Result<Self> {
        let url = service
            .base()
            .join(&format!("rest/bug?{}", query.params()?))?;
        Ok(Self { url, service })
    }
}

/// Advanced field matching operators.
#[derive(Display, Debug, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
enum MatchOp {
    /// case-sensitive substring matching
    CaseSubstring,
    /// case-insensitive substring matching
    Substring,
    /// inverted, case-insensitive substring matching
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

impl Match {
    fn equals<S: fmt::Display>(value: S) -> Self {
        Self {
            op: MatchOp::Equals,
            value: value.to_string(),
        }
    }
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
            Some(("is", value)) => (MatchOp::Substring, value.into()),
            Some(("s", value)) => (MatchOp::CaseSubstring, value.into()),
            Some(("!s", value)) => (MatchOp::NotSubstring, value.into()),
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

#[derive(Debug, Clone)]
pub enum EnabledOrDisabled<T> {
    Enabled(T),
    Disabled(T),
}

impl<T> FromStr for EnabledOrDisabled<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(value) = s.strip_prefix('-') {
            Ok(Self::Disabled(value.parse().map_err(|e| {
                Error::InvalidValue(format!("failed parsing: {e}"))
            })?))
        } else if let Some(value) = s.strip_prefix('+') {
            Ok(Self::Enabled(value.parse().map_err(|e| {
                Error::InvalidValue(format!("failed parsing: {e}"))
            })?))
        } else {
            Ok(Self::Enabled(s.parse().map_err(|e| {
                Error::InvalidValue(format!("failed parsing: {e}"))
            })?))
        }
    }
}

impl From<u64> for EnabledOrDisabled<u64> {
    fn from(value: u64) -> Self {
        Self::Enabled(value)
    }
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug)]
pub struct QueryBuilder<'a> {
    service: &'a super::Service,
    query: ListOrderedMultimap<String, String>,
    advanced_count: u64,
}

impl<'a> ServiceParams<'a> for QueryBuilder<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self {
            service,
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
        self.op_field("OR", "bug_id", values.into_iter().map(Match::equals));
    }

    pub fn alias<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "alias", values)
    }

    pub fn assignee<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "assigned_to", values);
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

    pub fn qa<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "qa_contact", values);
    }

    pub fn reporter<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "reporter", values);
    }

    pub fn resolution<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "resolution", values);
    }

    pub fn comment<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "longdesc", values)
    }

    pub fn summary<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "short_desc", values)
    }

    pub fn created<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = RangeOrValue<TimeDeltaIso8601>>,
    {
        for value in values {
            match value {
                RangeOrValue::Value(value) => {
                    self.advanced_field("creation_ts", "greaterthaneq", value)
                }
                RangeOrValue::RangeOp(value) => self.range_op("creation_ts", value),
                RangeOrValue::Range(value) => self.range("creation_ts", value),
            }
        }
    }

    pub fn modified<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = RangeOrValue<TimeDeltaIso8601>>,
    {
        for value in values {
            match value {
                RangeOrValue::Value(value) => {
                    self.advanced_field("delta_ts", "greaterthaneq", value)
                }
                RangeOrValue::RangeOp(value) => self.range_op("delta_ts", value),
                RangeOrValue::Range(value) => self.range("delta_ts", value),
            }
        }
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
        self.op_field("AND", "attachments.submitter", values)
    }

    pub fn commenters<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "commenter", values)
    }

    pub fn flaggers<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "setters.login_name", values)
    }

    pub fn url<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "bug_file_loc", values);
    }

    pub fn changed<'a, I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, &'a RangeOrValue<TimeDeltaIso8601>)>,
    {
        for (field, target) in values {
            let field = field.api();
            match target {
                RangeOrValue::Value(value) => self.advanced_field(field, "changedafter", value),
                RangeOrValue::RangeOp(value) => match value {
                    RangeOp::Less(value) => {
                        self.advanced_field(field, "changedbefore", value);
                    }
                    RangeOp::LessOrEqual(value) => {
                        self.advanced_field(field, "changedbefore", value);
                    }
                    RangeOp::Equal(value) => {
                        self.advanced_field(field, "equals", value);
                    }
                    RangeOp::NotEqual(value) => {
                        self.advanced_field(field, "notequals", value);
                    }
                    RangeOp::GreaterOrEqual(value) => {
                        self.advanced_field(field, "changedafter", value);
                    }
                    RangeOp::Greater(value) => {
                        self.advanced_field(field, "changedafter", value);
                    }
                },
                RangeOrValue::Range(value) => match value {
                    Range::Range(r) => {
                        self.advanced_field(field, "changedafter", &r.start);
                        self.advanced_field(field, "changedbefore", &r.end);
                    }
                    Range::Inclusive(r) => {
                        self.advanced_field(field, "changedafter", r.start());
                        self.advanced_field(field, "changedbefore", r.end());
                    }
                    Range::To(r) => {
                        self.advanced_field(field, "changedbefore", &r.end);
                    }
                    Range::ToInclusive(r) => {
                        self.advanced_field(field, "changedbefore", &r.end);
                    }
                    Range::From(r) => {
                        self.advanced_field(field, "changedafter", &r.start);
                    }
                    Range::Full(_) => (),
                },
            }
        }
    }

    pub fn changed_by<I, J, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, J)>,
        J: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for (field, users) in values {
            for user in users {
                let user = self.service.replace_user_alias(user.as_ref());
                self.advanced_field(field.api(), "changedby", user);
            }
        }
    }

    pub fn changed_from<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, S)>,
        S: fmt::Display,
    {
        for (field, value) in values {
            self.advanced_field(field.api(), "changedfrom", value);
        }
    }

    pub fn changed_to<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, S)>,
        S: fmt::Display,
    {
        for (field, value) in values {
            self.advanced_field(field.api(), "changedto", value);
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
            let value = value.into();
            self.advanced_field(name, value.op, value);
        }
    }

    pub fn priority<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "priority", values);
    }

    pub fn severity<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "bug_severity", values);
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
        S: Into<Match>,
    {
        self.op_field("OR", "version", values);
    }

    pub fn component<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "component", values)
    }

    pub fn product<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "product", values);
    }

    pub fn platform<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "platform", values);
    }

    pub fn os<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "op_sys", values);
    }

    pub fn see_also<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "see_also", values);
    }

    pub fn tags<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "tag", values);
    }

    pub fn target<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "target_milestone", values);
    }

    pub fn whiteboard<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "whiteboard", values);
    }

    pub fn votes<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = RangeOrValue<u64>>,
    {
        for value in values {
            match value {
                RangeOrValue::Value(value) => self.advanced_field("votes", "equals", value),
                RangeOrValue::RangeOp(value) => self.range_op("votes", value),
                RangeOrValue::Range(value) => self.range("votes", value),
            }
        }
    }

    pub fn comments<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = RangeOrValue<u64>>,
    {
        for value in values {
            match value {
                RangeOrValue::Value(value) => {
                    self.advanced_field("longdescs.count", "equals", value)
                }
                RangeOrValue::RangeOp(value) => self.range_op("longdescs.count", value),
                RangeOrValue::Range(value) => self.range("longdescs.count", value),
            }
        }
    }

    /// Match bugs with conditionally existent array field values.
    pub fn exists(&mut self, field: ExistsField, status: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        let status = if status { "isnotempty" } else { "isempty" };
        self.insert(format!("f{num}"), field.api());
        self.insert(format!("o{num}"), status);
    }

    pub fn blocks<I, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<EnabledOrDisabled<u64>>,
    {
        for value in values {
            match value.into() {
                EnabledOrDisabled::Enabled(value) => {
                    self.advanced_field("blocked", "equals", value)
                }
                EnabledOrDisabled::Disabled(value) => {
                    self.advanced_field("blocked", "notequals", value)
                }
            }
        }
    }

    pub fn depends<I, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<EnabledOrDisabled<u64>>,
    {
        for value in values {
            match value.into() {
                EnabledOrDisabled::Enabled(value) => {
                    self.advanced_field("dependson", "equals", value)
                }
                EnabledOrDisabled::Disabled(value) => {
                    self.advanced_field("dependson", "notequals", value)
                }
            }
        }
    }

    pub fn flags<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "flagtypes.name", values)
    }

    pub fn groups<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("OR", "bug_group", values);
    }

    pub fn keywords<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "keywords", values)
    }

    pub fn cc<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.op_field("AND", "cc", values)
    }

    pub fn fields<I, F>(&mut self, fields: I)
    where
        I: IntoIterator<Item = F>,
        F: Into<FilterField>,
    {
        let mut fields: IndexSet<_> = fields.into_iter().map(Into::into).collect();

        // always include bug IDs in field requests
        fields.insert(FilterField::Bug(BugField::Id));

        self.insert("include_fields", fields.iter().map(|f| f.api()).join(","));
    }

    fn range_op<T>(&mut self, field: &str, value: RangeOp<T>)
    where
        T: fmt::Display,
    {
        match value {
            RangeOp::Less(value) => {
                self.advanced_field(field, "lessthan", value);
            }
            RangeOp::LessOrEqual(value) => {
                self.advanced_field(field, "lessthaneq", value);
            }
            RangeOp::Equal(value) => {
                self.advanced_field(field, "equals", value);
            }
            RangeOp::NotEqual(value) => {
                self.advanced_field(field, "notequals", value);
            }
            RangeOp::GreaterOrEqual(value) => {
                self.advanced_field(field, "greaterthaneq", value);
            }
            RangeOp::Greater(value) => {
                self.advanced_field(field, "greaterthan", value);
            }
        }
    }

    fn range<T>(&mut self, field: &str, value: Range<T>)
    where
        T: fmt::Display,
    {
        match value {
            Range::Range(r) => {
                self.advanced_field(field, "greaterthaneq", r.start);
                self.advanced_field(field, "lessthan", r.end);
            }
            Range::Inclusive(r) => {
                self.advanced_field(field, "greaterthaneq", r.start());
                self.advanced_field(field, "lessthaneq", r.end());
            }
            Range::To(r) => {
                self.advanced_field(field, "lessthan", r.end);
            }
            Range::ToInclusive(r) => {
                self.advanced_field(field, "lessthaneq", r.end);
            }
            Range::From(r) => {
                self.advanced_field(field, "greaterthaneq", r.start);
            }
            Range::Full(_) => (),
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

    fn op<I, F, V>(&mut self, op: &str, values: I)
    where
        I: IntoIterator<Item = (F, V)>,
        F: fmt::Display,
        V: Into<Match>,
    {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), op);

        for (field, value) in values {
            let value = value.into();
            self.advanced_field(field, value.op, value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    fn op_field<F, I, S>(&mut self, op: &str, field: F, values: I)
    where
        F: fmt::Display + Copy,
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        let fields = iter::repeat_with(|| field);
        self.op(op, fields.zip(values))
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
            self.order(["+id"])?;
        }

        // limit requested fields by default to decrease bandwidth and speed up response
        if !self.query.contains_key("include_fields") {
            self.fields([BugField::Id, BugField::Summary]);
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
    Depends,
    Flags,
    Groups,
    Keywords,
    Qa,
    Tags,
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
            Self::Depends => "dependson",
            Self::Flags => "setters.login_name",
            Self::Groups => "bug_group",
            Self::Keywords => "keywords",
            Self::Qa => "qa_contact",
            Self::SeeAlso => "see_also",
            Self::Tags => "tag",
            Self::Url => "bug_file_loc",
            Self::Whiteboard => "status_whiteboard",
        }
    }
}

/// Valid search order sorting terms.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum OrderField {
    Alias,
    Assignee,
    Blocks,
    Comments,
    Component,
    Created,
    Deadline,
    Depends,
    Flags,
    Id,
    Keywords,
    LastVisit,
    Modified,
    Os,
    Platform,
    Priority,
    Product,
    Qa,
    Reporter,
    Resolution,
    Severity,
    Status,
    Summary,
    Tags,
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
            Self::Assignee => "assigned_to",
            Self::Blocks => "blocked",
            Self::Comments => "longdescs.count",
            Self::Component => "component",
            Self::Created => "creation_ts",
            Self::Deadline => "deadline",
            Self::Depends => "dependson",
            Self::Flags => "flagtypes.name",
            Self::Id => "bug_id",
            Self::Keywords => "keywords",
            Self::LastVisit => "last_visit_ts",
            Self::Modified => "delta_ts",
            Self::Os => "op_sys",
            Self::Platform => "platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Qa => "qa_contact",
            Self::Reporter => "reporter",
            Self::Resolution => "resolution",
            Self::Severity => "bug_severity",
            Self::Status => "bug_status",
            Self::Summary => "short_desc",
            Self::Tags => "tag",
            Self::Target => "target_milestone",
            Self::Url => "bug_file_loc",
            Self::Version => "version",
            Self::Votes => "votes",
            Self::Whiteboard => "status_whiteboard",
        }
    }
}

impl Api for Order<OrderField> {
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

/// Valid change fields.
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, Clone, Copy)]
#[strum(serialize_all = "kebab-case")]
pub enum ChangeField {
    Alias,
    Assignee,
    Blocks,
    Component,
    Cc,
    Deadline,
    Depends,
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
            Self::Assignee => "assigned_to",
            Self::Blocks => "blocked",
            Self::Component => "component",
            Self::Cc => "cc",
            Self::Deadline => "deadline",
            Self::Depends => "dependson",
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
