use std::fs;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::{fmt, iter};

use camino::Utf8Path;
use indexmap::IndexSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};

use crate::args::ExistsOrValues;
use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{self, Order};
use crate::service::bugzilla::Service;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{Api, InjectAuth, RequestSend, WebService};
use crate::utils::{or, prefix};
use crate::Error;

use super::{BugField, FilterField};

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    params: Parameters,
}

impl RequestSend for Request<'_> {
    type Output = Vec<Bug>;

    async fn send(self) -> crate::Result<Self::Output> {
        let params = self.params.encode(self.service)?;
        let url = self
            .service
            .config
            .base
            .join(&format!("rest/bug?{params}"))?;
        let request = self.service.client.get(url).auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let bugs = serde_json::from_value(data)
            .map_err(|e| Error::InvalidValue(format!("failed deserializing bugs: {e}")))?;
        Ok(bugs)
    }
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a Service) -> Self {
        Self {
            service,
            params: Default::default(),
        }
    }

    pub fn params(mut self, params: Parameters) -> Self {
        self.params = params;
        self
    }

    pub fn order<I>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = Order<OrderField>>,
    {
        self.params.order = Some(values.into_iter().collect());
        self
    }

    pub fn fields<I, F>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = F>,
        F: Into<FilterField>,
    {
        self.params.fields = Some(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn status<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.params.status = Some(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn summary<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Match>,
    {
        self.params.summary = Some(values.into_iter().map(Into::into).collect());
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

    pub fn limit(mut self, value: u64) -> Self {
        self.params.limit = Some(value);
        self
    }

    pub fn quicksearch<S: Into<String>>(mut self, value: S) -> Self {
        self.params.quicksearch = Some(value.into());
        self
    }
}

/// Advanced field matching operators.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MatchOp {
    /// Contains case-sensitive substring.
    CaseSubstring,
    /// Contains substring.
    Substring,
    /// Doesn't contain substring.
    NotSubstring,
    /// Equal to value.
    Equals,
    /// Not equal to value.
    NotEquals,
    /// Matches regular expression.
    Regexp,
    /// Doesn't match regular expression.
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
#[derive(DeserializeFromStr, SerializeDisplay, Debug, PartialEq, Eq, Clone)]
pub struct Match {
    op: MatchOp,
    value: String,
}

impl Match {
    /// Substitute user alias for matching value.
    fn replace_user_alias(mut self, service: &Service) -> Self {
        if let Some(user) = service.config.user.as_deref() {
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
        let (op, value) = match s.split_once(' ') {
            Some(("=~", value)) => (MatchOp::CaseSubstring, value.into()),
            Some(("~~", value)) => (MatchOp::Substring, value.into()),
            Some(("!~", value)) => (MatchOp::NotSubstring, value.into()),
            Some(("==", value)) => (MatchOp::Equals, value.into()),
            Some(("!=", value)) => (MatchOp::NotEquals, value.into()),
            Some(("=*", value)) => (MatchOp::Regexp, value.into()),
            Some(("!*", value)) => (MatchOp::NotRegexp, value.into()),
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

/// Bug search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Parameters {
    pub alias: Option<Vec<ExistsOrValues<Match>>>,
    pub attachments: Option<ExistsOrValues<Match>>,
    pub flags: Option<Vec<ExistsOrValues<Match>>>,
    pub groups: Option<Vec<ExistsOrValues<Match>>>,
    pub keywords: Option<Vec<ExistsOrValues<Match>>>,
    pub see_also: Option<Vec<ExistsOrValues<Match>>>,
    pub tags: Option<Vec<ExistsOrValues<Match>>>,
    pub whiteboard: Option<Vec<ExistsOrValues<Match>>>,
    pub url: Option<Vec<ExistsOrValues<Match>>>,

    pub attachment_description: Option<Vec<Vec<Match>>>,
    pub attachment_filename: Option<Vec<Vec<Match>>>,
    pub attachment_mime: Option<Vec<Vec<Match>>>,
    pub attachment_is_obsolete: Option<bool>,
    pub attachment_is_patch: Option<bool>,
    pub attachment_is_private: Option<bool>,

    pub changed: Option<Vec<(Vec<ChangeField>, RangeOrValue<TimeDeltaOrStatic>)>>,
    pub changed_by: Option<Vec<(Vec<ChangeField>, Vec<String>)>>,
    pub changed_from: Option<Vec<(ChangeField, String)>>,
    pub changed_to: Option<Vec<(ChangeField, String)>>,

    pub assignee: Option<Vec<Vec<Match>>>,
    pub attacher: Option<Vec<Vec<Match>>>,
    pub cc: Option<Vec<ExistsOrValues<Match>>>,
    pub commenter: Option<Vec<Vec<Match>>>,
    pub flagger: Option<Vec<Vec<Match>>>,
    pub qa: Option<Vec<ExistsOrValues<Match>>>,
    pub reporter: Option<Vec<Vec<Match>>>,

    #[serde(skip_serializing)]
    pub fields: Option<Vec<FilterField>>,
    pub limit: Option<u64>,
    pub order: Option<Vec<Order<OrderField>>>,

    pub created: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub updated: Option<RangeOrValue<TimeDeltaOrStatic>>,

    pub comment: Option<Vec<Match>>,
    pub comment_is_private: Option<bool>,
    pub comment_tag: Option<Vec<Vec<Match>>>,

    pub blocks: Option<Vec<ExistsOrValues<i64>>>,
    pub depends: Option<Vec<ExistsOrValues<i64>>>,
    pub ids: Option<Vec<RangeOrValue<i64>>>,
    pub priority: Option<Vec<Match>>,
    pub severity: Option<Vec<Match>>,
    pub version: Option<Vec<Match>>,
    pub component: Option<Vec<Match>>,
    pub product: Option<Vec<Match>>,
    pub platform: Option<Vec<Match>>,
    pub os: Option<Vec<Match>>,
    pub resolution: Option<Vec<Match>>,
    pub status: Option<Vec<String>>,
    pub target: Option<Vec<Match>>,
    pub comments: Option<RangeOrValue<u64>>,
    pub votes: Option<RangeOrValue<u64>>,
    pub summary: Option<Vec<Match>>,
    pub quicksearch: Option<String>,
    pub custom_fields: Option<Vec<(String, Match)>>,
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
        or!(self.alias, other.alias);
        or!(self.attachments, other.attachments);
        or!(self.flags, other.flags);
        or!(self.groups, other.groups);
        or!(self.keywords, other.keywords);
        or!(self.see_also, other.see_also);
        or!(self.tags, other.tags);
        or!(self.whiteboard, other.whiteboard);
        or!(self.url, other.url);

        or!(self.attachment_description, other.attachment_description);
        or!(self.attachment_filename, other.attachment_filename);
        or!(self.attachment_mime, other.attachment_mime);
        or!(self.attachment_is_obsolete, other.attachment_is_obsolete);
        or!(self.attachment_is_patch, other.attachment_is_patch);
        or!(self.attachment_is_private, other.attachment_is_private);

        or!(self.changed, other.changed);
        or!(self.changed_by, other.changed_by);
        or!(self.changed_from, other.changed_from);
        or!(self.changed_to, other.changed_to);

        or!(self.assignee, other.assignee);
        or!(self.attacher, other.attacher);
        or!(self.cc, other.cc);
        or!(self.commenter, other.commenter);
        or!(self.flagger, other.flagger);
        or!(self.qa, other.qa);
        or!(self.reporter, other.reporter);

        or!(self.fields, other.fields);
        or!(self.limit, other.limit);
        or!(self.order, other.order);

        or!(self.created, other.created);
        or!(self.updated, other.updated);

        or!(self.comment, other.comment);
        or!(self.comment_is_private, other.comment_is_private);
        or!(self.comment_tag, other.comment_tag);

        or!(self.blocks, other.blocks);
        or!(self.depends, other.depends);
        or!(self.ids, other.ids);
        or!(self.priority, other.priority);
        or!(self.severity, other.severity);
        or!(self.version, other.version);
        or!(self.component, other.component);
        or!(self.product, other.product);
        or!(self.platform, other.platform);
        or!(self.os, other.os);
        or!(self.resolution, other.resolution);
        or!(self.status, other.status);
        or!(self.target, other.target);
        or!(self.comments, other.comments);
        or!(self.votes, other.votes);
        or!(self.summary, other.summary);
        or!(self.quicksearch, other.quicksearch);
        or!(self.custom_fields, other.custom_fields);
    }

    pub(crate) fn encode(self, service: &Service) -> crate::Result<String> {
        let mut query = QueryBuilder::new(service);

        if let Some(values) = self.status {
            // separate aliases from values
            let (aliases, values): (Vec<_>, Vec<_>) =
                values.into_iter().partition(|x| x.starts_with('@'));

            // combine aliases via logical OR
            for value in aliases {
                query.status(value);
            }

            // combine values via logical OR
            if !values.is_empty() {
                query.or(|query| values.into_iter().for_each(|x| query.status(x)));
            }
        } else {
            // only return open bugs by default
            query.status("@open");
        }

        if let Some(values) = self.order {
            query.order(values)?;
        } else {
            // sort by ascending ID by default
            query.order([Order::Ascending(OrderField::Id)])?;
        }

        if let Some(values) = self.fields {
            query.fields(values);
        } else {
            // limit requested fields by default to decrease bandwidth and speed up response
            query.fields([BugField::Id, BugField::Summary]);
        }

        if let Some(value) = self.limit {
            query.insert("limit", value);
        }

        if let Some(values) = self.alias {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Alias, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.alias(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.attachments {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Attachments, value),
                ExistsOrValues::Values(values) => query.attachments(values),
            }
        }

        if let Some(values) = self.flags {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Flags, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.flags(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.groups {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Groups, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.groups(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.keywords {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Keywords, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.keywords(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.see_also {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::SeeAlso, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.see_also(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.tags {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Tags, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.tags(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.whiteboard {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => {
                            query.exists(ExistsField::Whiteboard, value)
                        }
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.whiteboard(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.url {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Url, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.url(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.changed {
            for (fields, interval) in values {
                query.changed(fields.into_iter().map(|f| (f, &interval)));
            }
        }

        if let Some(values) = self.changed_by {
            for (fields, users) in values {
                query.changed_by(fields.into_iter().map(|f| (f, &users)));
            }
        }

        if let Some(values) = self.changed_from {
            query.changed_from(values);
        }

        if let Some(values) = self.changed_to {
            query.changed_to(values);
        }

        if let Some(value) = self.comments {
            query.comments(value);
        }

        if let Some(value) = self.votes {
            query.votes(value);
        }

        if let Some(values) = self.assignee {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.assignee(x)))
                }
            });
        }

        if let Some(values) = self.attacher {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.attacher(x)))
                }
            });
        }

        if let Some(values) = self.cc {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Cc, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.cc(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.commenter {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.commenter(x)))
                }
            });
        }

        if let Some(values) = self.flagger {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.flagger(x)))
                }
            });
        }

        if let Some(values) = self.qa {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Qa, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.qa(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.reporter {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.reporter(x)))
                }
            });
        }

        if let Some(values) = self.comment {
            query.op_field("AND", "longdesc", values)
        }

        if let Some(value) = self.comment_is_private {
            query.comment_is_private(value);
        }

        if let Some(values) = self.comment_tag {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.comment_tag(x)))
                }
            });
        }

        if let Some(values) = self.summary {
            query.op_field("AND", "short_desc", values)
        }

        if let Some(values) = self.blocks {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocks, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.blocks(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.depends {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Depends, value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.into_iter().for_each(|x| query.depends(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = self.ids {
            query.or(|query| values.into_iter().for_each(|x| query.id(x)));
        }

        if let Some(values) = self.priority {
            query.or(|query| values.into_iter().for_each(|x| query.priority(x)));
        }

        if let Some(values) = self.severity {
            query.or(|query| values.into_iter().for_each(|x| query.severity(x)));
        }

        if let Some(values) = self.version {
            query.or(|query| values.into_iter().for_each(|x| query.version(x)));
        }

        if let Some(values) = self.component {
            query.or(|query| values.into_iter().for_each(|x| query.component(x)));
        }

        if let Some(values) = self.product {
            query.or(|query| values.into_iter().for_each(|x| query.product(x)));
        }

        if let Some(values) = self.platform {
            query.or(|query| values.into_iter().for_each(|x| query.platform(x)));
        }

        if let Some(values) = self.os {
            query.or(|query| values.into_iter().for_each(|x| query.os(x)));
        }

        if let Some(values) = self.resolution {
            query.or(|query| values.into_iter().for_each(|x| query.resolution(x)));
        }

        if let Some(values) = self.target {
            query.or(|query| values.into_iter().for_each(|x| query.target(x)));
        }

        if let Some(value) = self.created {
            query.created(value);
        }

        if let Some(value) = self.updated {
            query.updated(value);
        }

        if let Some(value) = self.quicksearch {
            query.insert("quicksearch", value);
        }

        if let Some(values) = self.custom_fields {
            query.custom_fields(values);
        }

        if let Some(values) = self.attachment_description {
            query.or(|query| {
                for value in values {
                    query.and(|query| {
                        value
                            .into_iter()
                            .for_each(|x| query.attachment_description(x))
                    })
                }
            });
        }

        if let Some(values) = self.attachment_filename {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.attachment_filename(x)))
                }
            });
        }

        if let Some(values) = self.attachment_mime {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.into_iter().for_each(|x| query.attachment_mime(x)))
                }
            });
        }

        if let Some(value) = self.attachment_is_obsolete {
            query.attachment_is_obsolete(value);
        }

        if let Some(value) = self.attachment_is_patch {
            query.attachment_is_patch(value);
        }

        if let Some(value) = self.attachment_is_private {
            query.attachment_is_private(value);
        }

        Ok(query.encode())
    }
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug)]
struct QueryBuilder<'a> {
    service: &'a Service,
    query: query::QueryBuilder,
    advanced_count: u64,
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
    fn new(service: &'a Service) -> Self {
        Self {
            service,
            query: Default::default(),
            advanced_count: Default::default(),
        }
    }
}

impl QueryBuilder<'_> {
    fn id(&mut self, value: RangeOrValue<i64>) {
        match value {
            RangeOrValue::Value(value) => {
                if value >= 0 {
                    self.advanced_field("bug_id", "equals", value);
                } else {
                    self.advanced_field("bug_id", "notequals", value.abs());
                }
            }
            RangeOrValue::RangeOp(value) => self.range_op("bug_id", value),
            RangeOrValue::Range(value) => self.range("bug_id", value),
        }
    }

    fn alias<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("alias", value.op, value);
    }

    fn assignee<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("assigned_to", value.op, value);
    }

    /// Search for attachments with matching descriptions or filenames.
    fn attachments<I, S>(&mut self, values: I)
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

    fn attachment_description<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("attachments.description", value.op, value);
    }

    fn attachment_filename<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("attachments.filename", value.op, value);
    }

    fn attachment_mime<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("attachments.mimetype", value.op, value);
    }

    fn attachment_is_obsolete(&mut self, value: bool) {
        self.boolean("attachments.isobsolete", value)
    }

    fn attachment_is_patch(&mut self, value: bool) {
        self.boolean("attachments.ispatch", value)
    }

    fn attachment_is_private(&mut self, value: bool) {
        self.boolean("attachments.isprivate", value)
    }

    fn comment_is_private(&mut self, value: bool) {
        self.boolean("longdescs.isprivate", value)
    }

    fn comment_tag<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("comment_tag", value.op, value);
    }

    fn qa<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("qa_contact", value.op, value);
    }

    fn reporter<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("reporter", value.op, value);
    }

    fn resolution<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("resolution", value.op, value);
    }

    fn created(&mut self, value: RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => {
                self.advanced_field("creation_ts", "greaterthaneq", value)
            }
            RangeOrValue::RangeOp(value) => self.range_op("creation_ts", value),
            RangeOrValue::Range(value) => self.range("creation_ts", value),
        }
    }

    fn updated(&mut self, value: RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("delta_ts", "greaterthaneq", value),
            RangeOrValue::RangeOp(value) => self.range_op("delta_ts", value),
            RangeOrValue::Range(value) => self.range("delta_ts", value),
        }
    }

    fn order<I, T>(&mut self, values: I) -> crate::Result<()>
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

    fn attacher<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("attachments.submitter", value.op, value);
    }

    fn commenter<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("commenter", value.op, value);
    }

    fn flagger<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("setters.login_name", value.op, value);
    }

    fn url<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("bug_file_loc", value.op, value);
    }

    fn changed<'a, I>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, &'a RangeOrValue<TimeDeltaOrStatic>)>,
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

    fn changed_by<I, J, S>(&mut self, values: I)
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

    fn changed_from<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, S)>,
        S: fmt::Display,
    {
        for (field, value) in values {
            self.advanced_field(field.api(), "changedfrom", value.to_string());
        }
    }

    fn changed_to<I, S>(&mut self, values: I)
    where
        I: IntoIterator<Item = (ChangeField, S)>,
        S: fmt::Display,
    {
        for (field, value) in values {
            self.advanced_field(field.api(), "changedto", value.to_string());
        }
    }

    fn custom_fields<I, K, V>(&mut self, values: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: fmt::Display,
        V: Into<Match>,
    {
        for (name, value) in values {
            let value = value.into();
            self.advanced_field(prefix!("cf_", name), value.op, value);
        }
    }

    fn priority<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("priority", value.op, value);
    }

    fn severity<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("bug_severity", value.op, value);
    }

    fn status<S: AsRef<str>>(&mut self, value: S) {
        // TODO: Consider reverting to converting aliases into regular values so
        // advanced fields can be used in all cases.
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

    fn version<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("version", value.op, value);
    }

    fn component<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("component", value.op, value);
    }

    fn product<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("product", value.op, value);
    }

    fn platform<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("platform", value.op, value);
    }

    fn os<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("op_sys", value.op, value);
    }

    fn see_also<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("see_also", value.op, value);
    }

    fn tags<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("tag", value.op, value);
    }

    fn target<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("target_milestone", value.op, value);
    }

    fn whiteboard<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("whiteboard", value.op, value);
    }

    fn votes(&mut self, value: RangeOrValue<u64>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("votes", "equals", value),
            RangeOrValue::RangeOp(value) => self.range_op("votes", value),
            RangeOrValue::Range(value) => self.range("votes", value),
        }
    }

    fn comments(&mut self, value: RangeOrValue<u64>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("longdescs.count", "equals", value),
            RangeOrValue::RangeOp(value) => self.range_op("longdescs.count", value),
            RangeOrValue::Range(value) => self.range("longdescs.count", value),
        }
    }

    /// Match bugs with conditionally existent array field values.
    fn exists(&mut self, field: ExistsField, status: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        let status = if status { "isnotempty" } else { "isempty" };
        self.insert(format!("f{num}"), field.api());
        self.insert(format!("o{num}"), status);
    }

    /// Match bugs with boolean field values.
    fn boolean<F: Api>(&mut self, field: F, status: bool) {
        let status = if status { 1 } else { 0 };
        self.advanced_field(field, "equals", status);
    }

    fn blocks(&mut self, value: i64) {
        if value >= 0 {
            self.advanced_field("blocked", "equals", value);
        } else {
            self.advanced_field("blocked", "notequals", value.abs());
        }
    }

    fn depends(&mut self, value: i64) {
        if value >= 0 {
            self.advanced_field("dependson", "equals", value);
        } else {
            self.advanced_field("dependson", "notequals", value.abs());
        }
    }

    fn flags<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("flagtypes.name", value.op, value)
    }

    fn groups<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("bug_group", value.op, value)
    }

    fn keywords<V: Into<Match>>(&mut self, value: V) {
        let value = value.into();
        self.advanced_field("keywords", value.op, value)
    }

    fn cc<V: Into<Match>>(&mut self, value: V) {
        let value = value.into().replace_user_alias(self.service);
        self.advanced_field("cc", value.op, value);
    }

    fn fields<I, F>(&mut self, fields: I)
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
        T: Api + Eq,
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
        T: Api + Eq,
    {
        match value {
            Range::Range(r) => {
                self.and(|query| {
                    query.advanced_field(field, "greaterthaneq", r.start);
                    query.advanced_field(field, "lessthan", r.end);
                });
            }
            Range::Inclusive(r) => self.and(|query| {
                query.advanced_field(field, "greaterthaneq", r.start());
                query.advanced_field(field, "lessthaneq", r.end());
            }),
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

    fn op_func<F: FnOnce(&mut Self)>(&mut self, op: &str, func: F) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), op);
        func(self);
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    fn or<F: FnOnce(&mut Self)>(&mut self, func: F) {
        self.op_func("OR", func)
    }

    fn and<F: FnOnce(&mut Self)>(&mut self, func: F) {
        self.op_func("AND", func)
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
#[derive(Display, EnumIter, EnumString, VariantNames, Debug, PartialEq, Eq, Clone, Copy)]
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
    Updated,
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
            Self::Created => "opendate",
            Self::Deadline => "deadline",
            Self::Depends => "dependson",
            Self::Flags => "flagtypes.name",
            Self::Id => "bug_id",
            Self::Keywords => "keywords",
            Self::LastVisit => "last_visit_ts",
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
            Self::Updated => "changeddate",
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
        match self {
            Order::Ascending(field) => format!("{} ASC", field.api()),
            Order::Descending(field) => format!("{} DESC", field.api()),
        }
    }
}

/// Valid change fields.
#[derive(
    Display,
    EnumIter,
    EnumString,
    VariantNames,
    DeserializeFromStr,
    SerializeDisplay,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
)]
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

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        server.respond(200, path.join("search/ids.json")).await;
        let bugs = service.search().summary(["test"]).send().await.unwrap();
        assert_eq!(bugs.len(), 5);
    }
}
