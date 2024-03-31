use std::collections::HashSet;
use std::str::FromStr;
use std::{fmt, iter};

use indexmap::IndexSet;
use itertools::Itertools;
use ordered_multimap::ListOrderedMultimap;
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, OrderType};
use crate::time::TimeDelta;
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
#[derive(Debug, Clone, Copy)]
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

impl Api for MatchOp {
    fn api(&self) -> String {
        let value = match self {
            Self::CaseSubstring => "casesubstring",
            Self::Substring => "substring",
            Self::NotSubstring => "notsubstring",
            Self::Equals => "equals",
            Self::NotEquals => "notequals",
            Self::Regexp => "regexp",
            Self::NotRegexp => "notregexp",
        };
        value.to_string()
    }
}

/// Advanced field match.
#[derive(Debug, Clone)]
pub struct Match {
    op: MatchOp,
    value: String,
}

impl Match {
    /// Substitute user alias for matching value.
    fn replace_user_alias(mut self, service: &super::Service) -> Self {
        if let Some(user) = service.user() {
            if self.value == "@me" {
                self.value = user.to_string();
            }
        }
        self
    }
}

impl Api for Match {
    fn api(&self) -> String {
        self.value.to_string()
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

impl From<&String> for Match {
    fn from(s: &String) -> Self {
        s.as_str().into()
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
    defaults: HashSet<String>,
}

impl<'a> ServiceParams<'a> for QueryBuilder<'a> {
    type Service = super::Service;

    fn new(service: &'a Self::Service) -> Self {
        Self {
            service,
            query: Default::default(),
            advanced_count: Default::default(),
            defaults: Default::default(),
        }
    }
}

impl QueryBuilder<'_> {
    pub fn id(&mut self, value: i64) {
        if value >= 0 {
            self.advanced_field("bug_id", "equals", value);
        } else {
            self.advanced_field("bug_id", "notequals", value.abs());
        }
    }

    pub fn alias<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("alias", value.op, value);
    }

    pub fn assignee<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("assigned_to", value.op, value);
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

    pub fn qa<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("qa_contact", value.op, value);
    }

    pub fn reporter<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("reporter", value.op, value);
    }

    pub fn resolution<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("resolution", value.op, value);
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

    pub fn created(&mut self, value: RangeOrValue<TimeDelta>) {
        match value {
            RangeOrValue::Value(value) => {
                self.advanced_field("creation_ts", "greaterthaneq", value)
            }
            RangeOrValue::RangeOp(value) => self.range_op("creation_ts", value),
            RangeOrValue::Range(value) => self.range("creation_ts", value),
        }
    }

    pub fn modified(&mut self, value: RangeOrValue<TimeDelta>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("delta_ts", "greaterthaneq", value),
            RangeOrValue::RangeOp(value) => self.range_op("delta_ts", value),
            RangeOrValue::Range(value) => self.range("delta_ts", value),
        }
    }

    pub fn order<I, T>(&mut self, values: I) -> crate::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: TryInto<Order<OrderField>>,
        <T as TryInto<Order<OrderField>>>::Error: std::fmt::Display,
    {
        // don't set order by default
        self.defaults.insert("order".to_string());

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

    pub fn attacher<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("attachments.submitter", value.op, value);
    }

    pub fn commenter<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("commenter", value.op, value);
    }

    pub fn flagger<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("setters.login_name", value.op, value);
    }

    pub fn url<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("bug_file_loc", value.op, value);
    }

    pub fn changed<'a, I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, &'a RangeOrValue<TimeDelta>)>,
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
                        self.advanced_field(&field, "changedafter", &r.start);
                        self.advanced_field(&field, "changedbefore", &r.end);
                    }
                    Range::Inclusive(r) => {
                        self.advanced_field(&field, "changedafter", r.start());
                        self.advanced_field(&field, "changedbefore", r.end());
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
            self.advanced_field(field.api(), "changedfrom", value.to_string());
        }
    }

    pub fn changed_to<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, S)>,
        S: fmt::Display,
    {
        for (field, value) in values {
            self.advanced_field(field.api(), "changedto", value.to_string());
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

    pub fn priority<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("priority", value.op, value);
    }

    pub fn severity<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("bug_severity", value.op, value);
    }

    pub fn status<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        // don't set status by default
        self.defaults.insert("status".to_string());

        for value in values {
            match value.as_ref() {
                "@open" => self.append("bug_status", "__open__"),
                "@closed" => self.append("bug_status", "__closed__"),
                "@all" => self.append("bug_status", "__all__"),
                value => {
                    if let Some(value) = value.strip_prefix('!') {
                        self.advanced_field("bug_status", "notequals", value)
                    } else {
                        self.advanced_field("bug_status", "equals", value)
                    }
                }
            }
        }
    }

    pub fn version<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("version", value.op, value);
    }

    pub fn component<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("component", value.op, value);
    }

    pub fn product<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("product", value.op, value);
    }

    pub fn platform<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("platform", value.op, value);
    }

    pub fn os<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("op_sys", value.op, value);
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

    pub fn target<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("target_milestone", value.op, value);
    }

    pub fn whiteboard<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("whiteboard", value.op, value);
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

    pub fn blocks(&mut self, value: i64) {
        if value >= 0 {
            self.advanced_field("blocked", "equals", value);
        } else {
            self.advanced_field("blocked", "notequals", value.abs());
        }
    }

    pub fn depends(&mut self, value: i64) {
        if value >= 0 {
            self.advanced_field("dependson", "equals", value);
        } else {
            self.advanced_field("dependson", "notequals", value.abs());
        }
    }

    pub fn flags<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("flagtypes.name", value.op, value)
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

    pub fn cc<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("cc", value.op, value);
    }

    pub fn fields<I, F>(&mut self, fields: I)
    where
        I: IntoIterator<Item = F>,
        F: Into<FilterField>,
    {
        // don't set fields by default
        self.defaults.insert("fields".to_string());

        let mut fields: IndexSet<_> = fields.into_iter().map(Into::into).collect();

        // always include bug IDs in field requests
        fields.insert(FilterField::Bug(BugField::Id));

        self.insert("include_fields", fields.iter().map(|f| f.api()).join(","));
    }

    fn range_op<T>(&mut self, field: &str, value: RangeOp<T>)
    where
        T: Api,
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
        T: Api,
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
        K: Api,
        V: Api,
    {
        self.query.append(key.api(), value.api());
    }

    fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Api,
        V: Api,
    {
        self.query.insert(key.api(), value.api());
    }

    fn advanced_field<F, K, V>(&mut self, field: F, operator: K, value: V)
    where
        F: Api,
        K: Api,
        V: Api,
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
        F: Api,
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
        F: Api + Copy,
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        let fields = iter::repeat_with(|| field);
        self.op(op, fields.zip(values))
    }

    pub fn op_func<F: FnOnce(&mut Self)>(&mut self, op: &str, func: F) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), op);
        func(self);
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    pub fn or<F: FnOnce(&mut Self)>(&mut self, func: F) {
        self.op_func("OR", func)
    }

    pub fn and<F: FnOnce(&mut Self)>(&mut self, func: F) {
        self.op_func("AND", func)
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
        if !self.defaults.contains("status") {
            self.status(["@open"]);
        }

        // sort by ascending ID by default
        if !self.defaults.contains("order") {
            self.order(["+id"])?;
        }

        // limit requested fields by default to decrease bandwidth and speed up response
        if !self.defaults.contains("fields") {
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
    fn api(&self) -> String {
        let value = match self {
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
        };
        value.to_string()
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
    fn api(&self) -> String {
        let value = match self {
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
        };
        value.to_string()
    }
}

impl Api for Order<OrderField> {
    fn api(&self) -> String {
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
    fn api(&self) -> String {
        let value = match self {
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
        };
        value.to_string()
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
