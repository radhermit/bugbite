use std::collections::HashSet;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use indexmap::IndexSet;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use strum::{AsRefStr, Display, EnumIter, EnumString};
use url::Url;

use crate::args::ExistsOrValues;
use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, Query};
use crate::service::bugzilla::Bugzilla;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{
    Api, InjectAuth, Merge, MergeOption, RequestStream, RequestTemplate, WebService,
};
use crate::Error;

use super::{BugField, FilterField};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request {
    #[serde(skip)]
    service: Bugzilla,
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
    type Item = Bug;

    fn concurrent(&self) -> Option<usize> {
        self.service.config.client.concurrent
    }

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

    async fn send(self) -> crate::Result<Vec<Bug>> {
        let mut url = self.service.config.base.join("rest/bug")?;
        let query = self.encode()?;
        url.query_pairs_mut().extend_pairs(query.iter());
        let request = self.service.client.get(url).auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let mut bugs = vec![];
        if let serde_json::Value::Array(values) = data["bugs"].take() {
            for value in values {
                let bug = self.service.deserialize_bug(value)?;
                bugs.push(bug);
            }
        }
        Ok(bugs)
    }
}

impl RequestTemplate for Request {
    type Params = Parameters;
    type Service = Bugzilla;
    const TYPE: &'static str = "search";

    fn service(&self) -> &Self::Service {
        &self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

impl Request {
    pub(super) fn new(service: &Bugzilla) -> Self {
        Self {
            service: service.clone(),
            params: Default::default(),
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(&self.service);

        if let Some(values) = &self.params.status {
            query.or(|query| values.iter().for_each(|x| query.status(x)));
        } else {
            // only return open bugs by default
            query.status("@open");
        }

        if let Some(values) = self.params.order.as_deref() {
            query.order(values);
        } else {
            // sort by ascending ID by default
            query.order(&[Order::Ascending(OrderField::Id)]);
        }

        if let Some(values) = &self.params.fields {
            query.fields(values.iter().copied());
        } else {
            // limit requested fields by default to decrease bandwidth and speed up response
            query.fields([BugField::Id, BugField::Summary]);
        }

        if let Some(value) = &self.params.limit {
            query.insert("limit", value);
        }

        if let Some(value) = &self.params.offset {
            query.insert("offset", value);
        }

        if let Some(values) = &self.params.alias {
            query.or(|query| {
                for value in values {
                    match value {
                        // HACK: Work around a server bug where regular "isempty" queries don't
                        // work with the alias field so use inverted existence queries instead for
                        // nonexistence.
                        ExistsOrValues::Exists(value) => {
                            query.not(!value, |query| query.exists(ExistsField::Alias, true))
                        }
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.alias(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.attachments {
            match values {
                ExistsOrValues::Exists(value) => query.exists(ExistsField::Attachments, *value),
                ExistsOrValues::Values(values) => query.attachments(values),
            }
        }

        if let Some(values) = &self.params.flags {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Flags, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.flags(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.groups {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Groups, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.groups(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.keywords {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => {
                            query.exists(ExistsField::Keywords, *value)
                        }
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.keywords(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.see_also {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::SeeAlso, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.see_also(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.tags {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Tags, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.tags(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.whiteboard {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => {
                            query.exists(ExistsField::Whiteboard, *value)
                        }
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.whiteboard(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.url {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Url, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.url(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.changed {
            for (fields, interval) in values {
                query.changed(fields.iter().map(|f| (f, interval)))?;
            }
        }

        if let Some(values) = &self.params.changed_by {
            for (fields, users) in values {
                query.changed_by(fields.iter().map(|f| (f, users)))?;
            }
        }

        if let Some(values) = &self.params.changed_from {
            query.changed_from(values)?;
        }

        if let Some(values) = &self.params.changed_to {
            query.changed_to(values)?;
        }

        if let Some(value) = &self.params.comments {
            query.comments(value);
        }

        if let Some(value) = &self.params.votes {
            query.votes(value);
        }

        if let Some(values) = &self.params.assignee {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.assignee(x)))
                }
            });
        }

        if let Some(values) = &self.params.attacher {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.attacher(x)))
                }
            });
        }

        if let Some(values) = &self.params.cc {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Cc, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.cc(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.commenter {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.commenter(x)))
                }
            });
        }

        if let Some(values) = &self.params.flagger {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.flagger(x)))
                }
            });
        }

        if let Some(values) = &self.params.qa {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Qa, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.qa(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.reporter {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.reporter(x)))
                }
            });
        }

        if let Some(values) = &self.params.comment {
            query.and(|query| values.iter().for_each(|x| query.comment(x)));
        }

        if let Some(value) = &self.params.comment_is_private {
            query.comment_is_private(*value);
        }

        if let Some(values) = &self.params.comment_tag {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.comment_tag(x)))
                }
            });
        }

        if let Some(values) = &self.params.summary {
            query.and(|query| values.iter().for_each(|x| query.summary(x)));
        }

        if let Some(values) = &self.params.blocks {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Blocks, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.blocks(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.depends {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(ExistsField::Depends, *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.depends(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.ids {
            query.or(|query| {
                for value in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists("bug_id", *value),
                        ExistsOrValues::Values(values) => {
                            query.and(|query| values.iter().for_each(|x| query.ids(x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.priority {
            query.or(|query| values.iter().for_each(|x| query.priority(x)));
        }

        if let Some(values) = &self.params.severity {
            query.or(|query| values.iter().for_each(|x| query.severity(x)));
        }

        if let Some(values) = &self.params.version {
            query.or(|query| values.iter().for_each(|x| query.version(x)));
        }

        if let Some(values) = &self.params.component {
            query.or(|query| values.iter().for_each(|x| query.component(x)));
        }

        if let Some(values) = &self.params.product {
            query.or(|query| values.iter().for_each(|x| query.product(x)));
        }

        if let Some(values) = &self.params.platform {
            query.or(|query| values.iter().for_each(|x| query.platform(x)));
        }

        if let Some(values) = &self.params.os {
            query.or(|query| values.iter().for_each(|x| query.os(x)));
        }

        if let Some(values) = &self.params.resolution {
            query.or(|query| values.iter().for_each(|x| query.resolution(x)));
        }

        if let Some(values) = &self.params.target {
            query.or(|query| values.iter().for_each(|x| query.target(x)));
        }

        if let Some(value) = &self.params.created {
            query.created(value);
        }

        if let Some(value) = &self.params.updated {
            query.updated(value);
        }

        if let Some(value) = &self.params.closed {
            query.changed([(StaticChangeField::Status, value)])?;
            query.status("@closed");
        }

        if let Some(value) = &self.params.quicksearch {
            query.insert("quicksearch", value);
        }

        if let Some(values) = &self.params.custom_fields {
            query.or(|query| {
                for (name, value) in values {
                    match value {
                        ExistsOrValues::Exists(value) => query.exists(name, *value),
                        ExistsOrValues::Values(values) => query
                            .and(|query| values.iter().for_each(|x| query.custom_field(name, x))),
                    }
                }
            });
        }

        if let Some(values) = &self.params.attachment_description {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.attachment_description(x)))
                }
            });
        }

        if let Some(values) = &self.params.attachment_filename {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.attachment_filename(x)))
                }
            });
        }

        if let Some(values) = &self.params.attachment_mime {
            query.or(|query| {
                for value in values {
                    query.and(|query| value.iter().for_each(|x| query.attachment_mime(x)))
                }
            });
        }

        if let Some(value) = &self.params.attachment_is_obsolete {
            query.attachment_is_obsolete(*value);
        }

        if let Some(value) = &self.params.attachment_is_patch {
            query.attachment_is_patch(*value);
        }

        if let Some(value) = &self.params.attachment_is_private {
            query.attachment_is_private(*value);
        }

        Ok(query)
    }

    /// Return the website URL for a query.
    pub fn search_url(&self) -> crate::Result<Url> {
        let mut url = self.service.config.base.join("buglist.cgi")?;
        let query = self.encode()?;
        url.query_pairs_mut().extend_pairs(query.iter());
        Ok(url)
    }

    pub fn alias<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .alias
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn attachments<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        self.params.attachments = Some(value.into());
        self
    }

    pub fn flags<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .flags
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn groups<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .groups
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn keywords<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .keywords
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn see_also<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .see_also
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn tags<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .tags
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn whiteboard<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .whiteboard
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn url<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .url
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn changed<F>(mut self, field: F) -> Self
    where
        F: fmt::Display,
    {
        self.params
            .changed
            .get_or_insert_with(Default::default)
            .push((vec![field.to_string()], "<now".parse().unwrap()));
        self
    }

    pub fn changed_at<F>(mut self, field: F, value: RangeOrValue<TimeDeltaOrStatic>) -> Self
    where
        F: fmt::Display,
    {
        self.params
            .changed
            .get_or_insert_with(Default::default)
            .push((vec![field.to_string()], value));
        self
    }

    pub fn changed_by<F, I, S>(mut self, field: F, values: I) -> Self
    where
        F: fmt::Display,
        I: IntoIterator<Item = S>,
        S: fmt::Display,
    {
        let values = values.into_iter().map(|x| x.to_string()).collect();
        self.params
            .changed_by
            .get_or_insert_with(Default::default)
            .push((vec![field.to_string()], values));
        self
    }

    pub fn changed_from<F, S>(mut self, field: F, value: S) -> Self
    where
        F: fmt::Display,
        S: fmt::Display,
    {
        self.params
            .changed_from
            .get_or_insert_with(Default::default)
            .push((field.to_string(), value.to_string()));
        self
    }

    pub fn changed_to<F, S>(mut self, field: F, value: S) -> Self
    where
        F: fmt::Display,
        S: fmt::Display,
    {
        self.params
            .changed_to
            .get_or_insert_with(Default::default)
            .push((field.to_string(), value.to_string()));
        self
    }

    pub fn assignee<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Match>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .assignee
            .get_or_insert_with(Default::default)
            .push(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn attacher<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Match>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .attacher
            .get_or_insert_with(Default::default)
            .push(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn commenter<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Match>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .commenter
            .get_or_insert_with(Default::default)
            .push(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn cc<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .cc
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn flagger<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Match>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .flagger
            .get_or_insert_with(Default::default)
            .push(values.into_iter().map(Into::into).collect());
        self
    }

    pub fn qa<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<Match>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .qa
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn reporter<I, T>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Match>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .reporter
            .get_or_insert_with(Default::default)
            .push(values.into_iter().map(Into::into).collect());
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

    pub fn blocks<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<RangeOrValue<i64>>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .blocks
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn depends<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<RangeOrValue<i64>>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .depends
            .get_or_insert_with(Default::default)
            .push(value.into());
        self
    }

    pub fn ids<T>(mut self, value: T) -> Self
    where
        T: Into<ExistsOrValues<RangeOrValue<i64>>>,
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

    pub fn limit(mut self, value: usize) -> Self {
        self.params.limit = Some(value);
        self
    }

    pub fn offset(mut self, value: usize) -> Self {
        self.params.offset = Some(value);
        self
    }

    pub fn quicksearch<S: Into<String>>(mut self, value: S) -> Self {
        self.params.quicksearch = Some(value.into());
        self
    }
}

/// Bug search parameters.
#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
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

    pub changed: Option<Vec<(Vec<String>, RangeOrValue<TimeDeltaOrStatic>)>>,
    pub changed_by: Option<Vec<(Vec<String>, Vec<String>)>>,
    pub changed_from: Option<Vec<(String, String)>>,
    pub changed_to: Option<Vec<(String, String)>>,

    pub assignee: Option<Vec<Vec<Match>>>,
    pub attacher: Option<Vec<Vec<Match>>>,
    pub cc: Option<Vec<ExistsOrValues<Match>>>,
    pub commenter: Option<Vec<Vec<Match>>>,
    pub flagger: Option<Vec<Vec<Match>>>,
    pub qa: Option<Vec<ExistsOrValues<Match>>>,
    pub reporter: Option<Vec<Vec<Match>>>,

    #[serde(skip_serializing)]
    pub fields: Option<Vec<FilterField>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order: Option<Vec<Order<OrderField>>>,
    pub paged: Option<bool>,

    pub created: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub updated: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub closed: Option<RangeOrValue<TimeDeltaOrStatic>>,

    pub comment: Option<Vec<Match>>,
    pub comment_is_private: Option<bool>,
    pub comment_tag: Option<Vec<Vec<Match>>>,

    pub blocks: Option<Vec<ExistsOrValues<RangeOrValue<i64>>>>,
    pub depends: Option<Vec<ExistsOrValues<RangeOrValue<i64>>>>,
    pub ids: Option<Vec<ExistsOrValues<RangeOrValue<i64>>>>,
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
    pub custom_fields: Option<Vec<(String, ExistsOrValues<Match>)>>,
}

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            alias: self.alias.merge(other.alias),
            attachments: self.attachments.merge(other.attachments),
            flags: self.flags.merge(other.flags),
            groups: self.groups.merge(other.groups),
            keywords: self.keywords.merge(other.keywords),
            see_also: self.see_also.merge(other.see_also),
            tags: self.tags.merge(other.tags),
            whiteboard: self.whiteboard.merge(other.whiteboard),
            url: self.url.merge(other.url),

            attachment_description: self
                .attachment_description
                .merge(other.attachment_description),
            attachment_filename: self.attachment_filename.merge(other.attachment_filename),
            attachment_mime: self.attachment_mime.merge(other.attachment_mime),
            attachment_is_obsolete: self
                .attachment_is_obsolete
                .merge(other.attachment_is_obsolete),
            attachment_is_patch: self.attachment_is_patch.merge(other.attachment_is_patch),
            attachment_is_private: self
                .attachment_is_private
                .merge(other.attachment_is_private),

            changed: self.changed.merge(other.changed),
            changed_by: self.changed_by.merge(other.changed_by),
            changed_from: self.changed_from.merge(other.changed_from),
            changed_to: self.changed_to.merge(other.changed_to),

            assignee: self.assignee.merge(other.assignee),
            attacher: self.attacher.merge(other.attacher),
            cc: self.cc.merge(other.cc),
            commenter: self.commenter.merge(other.commenter),
            flagger: self.flagger.merge(other.flagger),
            qa: self.qa.merge(other.qa),
            reporter: self.reporter.merge(other.reporter),

            fields: self.fields.merge(other.fields),
            limit: self.limit.merge(other.limit),
            offset: self.offset.merge(other.offset),
            order: self.order.merge(other.order),
            paged: self.paged.merge(other.paged),

            created: self.created.merge(other.created),
            updated: self.updated.merge(other.updated),
            closed: self.closed.merge(other.closed),

            comment: self.comment.merge(other.comment),
            comment_is_private: self.comment_is_private.merge(other.comment_is_private),
            comment_tag: self.comment_tag.merge(other.comment_tag),

            blocks: self.blocks.merge(other.blocks),
            depends: self.depends.merge(other.depends),
            ids: self.ids.merge(other.ids),
            priority: self.priority.merge(other.priority),
            severity: self.severity.merge(other.severity),
            version: self.version.merge(other.version),
            component: self.component.merge(other.component),
            product: self.product.merge(other.product),
            platform: self.platform.merge(other.platform),
            os: self.os.merge(other.os),
            resolution: self.resolution.merge(other.resolution),
            status: self.status.merge(other.status),
            target: self.target.merge(other.target),
            comments: self.comments.merge(other.comments),
            votes: self.votes.merge(other.votes),
            summary: self.summary.merge(other.summary),
            quicksearch: self.quicksearch.merge(other.quicksearch),
            custom_fields: self.custom_fields.merge(other.custom_fields),
        }
    }
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug)]
struct QueryBuilder<'a> {
    service: &'a Bugzilla,
    query: Query,
    advanced_count: u64,
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
    fn new(service: &'a Bugzilla) -> Self {
        Self {
            service,
            query: Default::default(),
            advanced_count: Default::default(),
        }
    }
}

/// Advanced field matching operators.
#[derive(Display, EnumIter, EnumString, Debug, Default, PartialEq, Eq, Clone, Copy)]
enum MatchOp {
    /// Contains case-sensitive substring.
    #[strum(serialize = "=~")]
    CaseSubstring,
    /// Contains substring.
    #[default]
    #[strum(serialize = "~~")]
    Substring,
    /// Doesn't contain substring.
    #[strum(serialize = "!~")]
    NotSubstring,
    /// Equal to value.
    #[strum(serialize = "==")]
    Equals,
    /// Not equal to value.
    #[strum(serialize = "!=")]
    NotEquals,
    /// Matches regular expression.
    #[strum(serialize = "=*")]
    Regexp,
    /// Doesn't match regular expression.
    #[strum(serialize = "!*")]
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
    op: Option<MatchOp>,
    value: String,
}

impl Match {
    /// Substitute user alias for matching value.
    fn replace_user_alias(&self, service: &Bugzilla) -> Self {
        Self {
            op: self.op,
            value: service.replace_user_alias(&self.value).to_string(),
        }
    }

    fn op(&self) -> MatchOp {
        self.op.unwrap_or_default()
    }
}

impl Api for Match {
    fn api(&self) -> String {
        self.value.to_string()
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(op) = &self.op {
            write!(f, "{op} ")?;
        }
        write!(f, "{}", self.value)
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
        let values = s.split_once(' ').map(|(op, value)| (op.parse(), value));

        let (op, value) = if let Some((Ok(op), value)) = values {
            (Some(op), value.into())
        } else {
            (None, s.into())
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

impl<T> From<bool> for ExistsOrValues<T> {
    fn from(value: bool) -> Self {
        ExistsOrValues::Exists(value)
    }
}

impl<T> From<T> for ExistsOrValues<Match>
where
    T: Into<Match>,
{
    fn from(value: T) -> Self {
        ExistsOrValues::Values(vec![value.into()])
    }
}

impl From<i64> for ExistsOrValues<RangeOrValue<i64>> {
    fn from(value: i64) -> Self {
        ExistsOrValues::Values(vec![value.into()])
    }
}

macro_rules! make_exists_or_values_range {
    ($($x:ty),+) => {$(
        impl<T: Eq> From<$x> for ExistsOrValues<RangeOrValue<T>> {
            fn from(value: $x) -> Self {
                ExistsOrValues::Values(vec![value.into()])
            }
        }
    )+};
}
make_exists_or_values_range!(
    std::ops::Range<T>,
    std::ops::RangeInclusive<T>,
    std::ops::RangeTo<T>,
    std::ops::RangeToInclusive<T>,
    std::ops::RangeFrom<T>,
    std::ops::RangeFull
);

macro_rules! make_exists_or_values_match_ref {
    ($($x:ty),+) => {$(
        impl From<$x> for ExistsOrValues<Match> {
            fn from(values: $x) -> Self {
                ExistsOrValues::Values(values.iter().copied().map(Into::into).collect())
            }
        }
    )+};
}
make_exists_or_values_match_ref!(&[&str], &Vec<&str>, &HashSet<&str>, &IndexSet<&str>);
make_exists_or_values_match_ref!(
    &[&String],
    &Vec<&String>,
    &HashSet<&String>,
    &IndexSet<&String>
);

macro_rules! make_exists_or_values_match_owned {
    ($($x:ty),+) => {$(
        impl From<$x> for ExistsOrValues<Match> {
            fn from(values: $x) -> Self {
                ExistsOrValues::Values(values.into_iter().map(Into::into).collect())
            }
        }
    )+};
}
make_exists_or_values_match_owned!(&[String], &Vec<String>, &HashSet<String>, &IndexSet<String>);

macro_rules! make_exists_or_values_i64 {
    ($($x:ty),+) => {$(
        impl From<$x> for ExistsOrValues<RangeOrValue<i64>> {
            fn from(values: $x) -> Self {
                ExistsOrValues::Values(values.iter().copied().map(Into::into).collect())
            }
        }
    )+};
}
make_exists_or_values_i64!(&[i64], &Vec<i64>, &HashSet<i64>, &IndexSet<i64>);

impl<T, const N: usize> From<&[T; N]> for ExistsOrValues<Match>
where
    T: Into<Match> + Copy,
{
    fn from(values: &[T; N]) -> Self {
        ExistsOrValues::Values(values.iter().copied().map(Into::into).collect())
    }
}

impl<const N: usize> From<&[i64; N]> for ExistsOrValues<RangeOrValue<i64>> {
    fn from(values: &[i64; N]) -> Self {
        ExistsOrValues::Values(values.iter().copied().map(Into::into).collect())
    }
}

impl<T, const N: usize> From<[T; N]> for ExistsOrValues<Match>
where
    T: Into<Match>,
{
    fn from(values: [T; N]) -> Self {
        ExistsOrValues::Values(values.into_iter().map(Into::into).collect())
    }
}

impl<const N: usize> From<[i64; N]> for ExistsOrValues<RangeOrValue<i64>> {
    fn from(values: [i64; N]) -> Self {
        ExistsOrValues::Values(values.into_iter().map(Into::into).collect())
    }
}

impl QueryBuilder<'_> {
    fn ids(&mut self, value: &RangeOrValue<i64>) {
        match value {
            RangeOrValue::Value(value) => {
                if *value >= 0 {
                    self.advanced_field("bug_id", "equals", value);
                } else {
                    self.advanced_field("bug_id", "notequals", value.abs());
                }
            }
            RangeOrValue::RangeOp(value) => self.range_op("bug_id", value),
            RangeOrValue::Range(value) => self.range("bug_id", value),
        }
    }

    fn alias(&mut self, value: &Match) {
        self.advanced_field("alias", value.op(), value);
    }

    fn assignee(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("assigned_to", value.op(), value);
    }

    /// Search for attachments with matching descriptions or filenames.
    fn attachments(&mut self, values: &[Match]) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), "OR");

        for value in values {
            self.advanced_field("attachments.description", value.op(), value);
            self.advanced_field("attachments.filename", value.op(), value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    fn attachment_description(&mut self, value: &Match) {
        self.advanced_field("attachments.description", value.op(), value);
    }

    fn attachment_filename(&mut self, value: &Match) {
        self.advanced_field("attachments.filename", value.op(), value);
    }

    fn attachment_mime(&mut self, value: &Match) {
        self.advanced_field("attachments.mimetype", value.op(), value);
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

    fn comment(&mut self, value: &Match) {
        self.advanced_field("longdesc", value.op(), value);
    }

    fn comment_is_private(&mut self, value: bool) {
        self.boolean("longdescs.isprivate", value)
    }

    fn comment_tag(&mut self, value: &Match) {
        self.advanced_field("comment_tag", value.op(), value);
    }

    fn qa(&mut self, value: &Match) {
        self.advanced_field("qa_contact", value.op(), value);
    }

    fn reporter(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("reporter", value.op(), value);
    }

    fn resolution(&mut self, value: &Match) {
        self.advanced_field("resolution", value.op(), value);
    }

    fn created(&mut self, value: &RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => {
                self.advanced_field("creation_ts", "greaterthaneq", value)
            }
            RangeOrValue::RangeOp(value) => self.range_op("creation_ts", value),
            RangeOrValue::Range(value) => self.range("creation_ts", value),
        }
    }

    fn updated(&mut self, value: &RangeOrValue<TimeDeltaOrStatic>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("delta_ts", "greaterthaneq", value),
            RangeOrValue::RangeOp(value) => self.range_op("delta_ts", value),
            RangeOrValue::Range(value) => self.range("delta_ts", value),
        }
    }

    fn order(&mut self, values: &[Order<OrderField>]) {
        let value = values.iter().map(|x| x.api()).join(",");
        self.insert("order", value);
    }

    fn attacher(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("attachments.submitter", value.op(), value);
    }

    fn commenter(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("commenter", value.op(), value);
    }

    fn flagger(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("setters.login_name", value.op(), value);
    }

    fn url(&mut self, value: &Match) {
        self.advanced_field("bug_file_loc", value.op(), value);
    }

    fn changed<'a, F, I>(&mut self, values: I) -> crate::Result<()>
    where
        F: AsRef<str>,
        I: IntoIterator<Item = (F, &'a RangeOrValue<TimeDeltaOrStatic>)>,
    {
        for (field, target) in values {
            let (status, field) = ChangeField::invertable(field)?;
            match target {
                RangeOrValue::Value(value) => self.not(status, |query| {
                    query.advanced_field(field, "changedafter", value)
                }),
                RangeOrValue::RangeOp(value) => match value {
                    RangeOp::Less(value) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedbefore", value)
                        });
                    }
                    RangeOp::LessOrEqual(value) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedbefore", value)
                        });
                    }
                    // TODO: use more specific Range type that doesn't include equality ops
                    RangeOp::Equal(_) => {
                        return Err(Error::InvalidValue(format!(
                            "equality operator invalid for change values: {target}"
                        )))
                    }
                    RangeOp::NotEqual(_) => {
                        return Err(Error::InvalidValue(format!(
                            "equality operator invalid for change values: {target}"
                        )))
                    }
                    RangeOp::GreaterOrEqual(value) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedafter", value);
                        });
                    }
                    RangeOp::Greater(value) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedafter", value);
                        });
                    }
                },
                RangeOrValue::Range(value) => match value {
                    Range::Range(r) => {
                        self.not(status, |query| {
                            query.advanced_field(&field, "changedafter", &r.start);
                            query.advanced_field(&field, "changedbefore", &r.end);
                        });
                    }
                    Range::Inclusive(r) => {
                        self.not(status, |query| {
                            query.advanced_field(&field, "changedafter", r.start());
                            query.advanced_field(&field, "changedbefore", r.end());
                        });
                    }
                    Range::To(r) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedbefore", &r.end)
                        });
                    }
                    Range::ToInclusive(r) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedbefore", &r.end);
                        });
                    }
                    Range::From(r) => {
                        self.not(status, |query| {
                            query.advanced_field(field, "changedafter", &r.start);
                        });
                    }
                    Range::Full(_) => {
                        let value = TimeDeltaOrStatic::from_str("now").unwrap();
                        self.not(status, |query| {
                            query.advanced_field(field, "changedbefore", value)
                        });
                    }
                },
            }
        }
        Ok(())
    }

    fn changed_by<F, I, J, S>(&mut self, values: I) -> crate::Result<()>
    where
        F: AsRef<str>,
        I: IntoIterator<Item = (F, J)>,
        J: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for (field, users) in values {
            let field = ChangeField::from_str(field.as_ref())?;
            for user in users {
                let user = self.service.replace_user_alias(user.as_ref());
                self.advanced_field(&field, "changedby", user);
            }
        }
        Ok(())
    }

    fn changed_from<'a, F, I, S>(&mut self, values: I) -> crate::Result<()>
    where
        F: AsRef<str> + 'a,
        I: IntoIterator<Item = &'a (F, S)>,
        S: Api + 'a,
    {
        for (field, value) in values {
            let field = ChangeField::from_str(field.as_ref())?;
            self.advanced_field(field, "changedfrom", value);
        }
        Ok(())
    }

    fn changed_to<'a, F, I, S>(&mut self, values: I) -> crate::Result<()>
    where
        F: AsRef<str> + 'a,
        I: IntoIterator<Item = &'a (F, S)>,
        S: Api + 'a,
    {
        for (field, value) in values {
            let field = ChangeField::from_str(field.as_ref())?;
            self.advanced_field(field, "changedto", value);
        }
        Ok(())
    }

    fn custom_field<F: Api>(&mut self, name: F, value: &Match) {
        self.advanced_field(name, value.op(), value);
    }

    fn priority(&mut self, value: &Match) {
        self.advanced_field("priority", value.op(), value);
    }

    fn severity(&mut self, value: &Match) {
        self.advanced_field("bug_severity", value.op(), value);
    }

    fn status<S: AsRef<str>>(&mut self, value: S) {
        // TODO: Consider reverting to converting aliases into regular values so
        // advanced fields can be used in all cases and multiple values can be appended.
        match value.as_ref() {
            "@open" => self.insert("bug_status", "__open__"),
            "@closed" => self.insert("bug_status", "__closed__"),
            "@all" => self.insert("bug_status", "__all__"),
            value => {
                if let Some(value) = value.strip_prefix('!') {
                    self.advanced_field("bug_status", "notequals", value)
                } else {
                    self.advanced_field("bug_status", "equals", value)
                }
            }
        }
    }

    fn version(&mut self, value: &Match) {
        self.advanced_field("version", value.op(), value);
    }

    fn component(&mut self, value: &Match) {
        self.advanced_field("component", value.op(), value);
    }

    fn product(&mut self, value: &Match) {
        self.advanced_field("product", value.op(), value);
    }

    fn platform(&mut self, value: &Match) {
        self.advanced_field("platform", value.op(), value);
    }

    fn os(&mut self, value: &Match) {
        self.advanced_field("op_sys", value.op(), value);
    }

    fn see_also(&mut self, value: &Match) {
        self.advanced_field("see_also", value.op(), value);
    }

    fn summary(&mut self, value: &Match) {
        self.advanced_field("short_desc", value.op(), value);
    }

    fn tags(&mut self, value: &Match) {
        self.advanced_field("tag", value.op(), value);
    }

    fn target(&mut self, value: &Match) {
        self.advanced_field("target_milestone", value.op(), value);
    }

    fn whiteboard(&mut self, value: &Match) {
        self.advanced_field("whiteboard", value.op(), value);
    }

    fn votes(&mut self, value: &RangeOrValue<u64>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("votes", "equals", value),
            RangeOrValue::RangeOp(value) => self.range_op("votes", value),
            RangeOrValue::Range(value) => self.range("votes", value),
        }
    }

    fn comments(&mut self, value: &RangeOrValue<u64>) {
        match value {
            RangeOrValue::Value(value) => self.advanced_field("longdescs.count", "equals", value),
            RangeOrValue::RangeOp(value) => self.range_op("longdescs.count", value),
            RangeOrValue::Range(value) => self.range("longdescs.count", value),
        }
    }

    /// Match bugs with conditionally existent array field values.
    fn exists<F: Api>(&mut self, field: F, status: bool) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        let status = if status { "isnotempty" } else { "isempty" };
        self.insert(format!("f{num}"), field);
        self.insert(format!("o{num}"), status);
    }

    /// Match bugs with boolean field values.
    fn boolean<F: Api>(&mut self, field: F, status: bool) {
        self.advanced_field(field, "equals", status as u64);
    }

    fn blocks(&mut self, value: &RangeOrValue<i64>) {
        match value {
            RangeOrValue::Value(value) => {
                if *value >= 0 {
                    self.advanced_field("blocked", "equals", value);
                } else {
                    self.advanced_field("blocked", "notequals", value.abs());
                }
            }
            RangeOrValue::RangeOp(value) => self.range_op("blocked", value),
            RangeOrValue::Range(value) => self.range("blocked", value),
        }
    }

    fn depends(&mut self, value: &RangeOrValue<i64>) {
        match value {
            RangeOrValue::Value(value) => {
                if *value >= 0 {
                    self.advanced_field("dependson", "equals", value);
                } else {
                    self.advanced_field("dependson", "notequals", value.abs());
                }
            }
            RangeOrValue::RangeOp(value) => self.range_op("dependson", value),
            RangeOrValue::Range(value) => self.range("dependson", value),
        }
    }

    fn flags(&mut self, value: &Match) {
        self.advanced_field("flagtypes.name", value.op(), value)
    }

    fn groups(&mut self, value: &Match) {
        self.advanced_field("bug_group", value.op(), value)
    }

    fn keywords(&mut self, value: &Match) {
        self.advanced_field("keywords", value.op(), value)
    }

    fn cc(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("cc", value.op(), value);
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

    fn range_op<T>(&mut self, field: &str, value: &RangeOp<T>)
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

    fn range<T>(&mut self, field: &str, value: &Range<T>)
    where
        T: Api + Eq,
    {
        match value {
            Range::Range(r) => {
                self.and(|query| {
                    query.advanced_field(field, "greaterthaneq", &r.start);
                    query.advanced_field(field, "lessthan", &r.end);
                });
            }
            Range::Inclusive(r) => self.and(|query| {
                query.advanced_field(field, "greaterthaneq", r.start());
                query.advanced_field(field, "lessthaneq", r.end());
            }),
            Range::To(r) => {
                self.advanced_field(field, "lessthan", &r.end);
            }
            Range::ToInclusive(r) => {
                self.advanced_field(field, "lessthaneq", &r.end);
            }
            Range::From(r) => {
                self.advanced_field(field, "greaterthaneq", &r.start);
            }
            Range::Full(_) => (),
        }
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

    fn not<F: FnOnce(&mut Self)>(&mut self, status: bool, func: F) {
        func(self);
        if status {
            let num = self.advanced_count;
            self.insert(format!("n{num}"), "1");
        }
    }
}

/// Bug fields composed of value arrays.
#[derive(Display, EnumIter, EnumString, Debug, Clone, Copy)]
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
#[derive(Display, EnumIter, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
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

/// Valid static change fields.
#[derive(AsRefStr, Display, EnumIter, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum StaticChangeField {
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
    Qa,
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

impl Api for StaticChangeField {
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
            Self::Platform => "rep_platform",
            Self::Priority => "priority",
            Self::Product => "product",
            Self::Qa => "qa_contact",
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

/// Valid change fields.
pub enum ChangeField {
    Static(StaticChangeField),
    Custom(String),
}

impl FromStr for ChangeField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("cf_") {
            Ok(Self::Custom(s.to_string()))
        } else {
            s.parse()
                .map(Self::Static)
                .map_err(|_| Error::InvalidValue(format!("invalid change field: {s}")))
        }
    }
}

impl Api for ChangeField {
    fn api(&self) -> String {
        match self {
            Self::Static(value) => value.api(),
            Self::Custom(value) => value.api(),
        }
    }
}

impl ChangeField {
    /// Parse invertable values, prefixing a field name with `!` inverts a query.
    fn invertable<S: AsRef<str>>(field: S) -> crate::Result<(bool, Self)> {
        let field = field.as_ref();
        match field.strip_prefix('!') {
            Some(value) => Ok((true, ChangeField::from_str(value)?)),
            None => Ok((false, ChangeField::from_str(field)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::service::bugzilla::GroupField;
    use crate::test::*;

    use super::*;

    // ExistsOrValues<Match> trait conversion testing
    #[tokio::test]
    async fn exists_or_values_match() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();
        server.respond(200, path.join("search/ids.json")).await;

        // boolean
        service.search().alias(true).send().await.unwrap();
        service.search().alias(false).send().await.unwrap();

        // string
        let value = "value".to_string();
        service.search().alias("value").send().await.unwrap();
        service.search().alias(&value).send().await.unwrap();
        service.search().alias(value).send().await.unwrap();

        // array
        service
            .search()
            .alias(["value1", "value2"])
            .send()
            .await
            .unwrap();

        // vector str
        let values = vec!["value1", "value2"];
        service.search().alias(&values).send().await.unwrap();
        service
            .search()
            .alias(values.as_slice())
            .send()
            .await
            .unwrap();
        // vector owned
        let values: Vec<_> = values.iter().map(|x| x.to_string()).collect();
        service.search().alias(&values).send().await.unwrap();
        service
            .search()
            .alias(values.as_slice())
            .send()
            .await
            .unwrap();
        // vector ref
        let values: Vec<_> = values.iter().collect();
        service.search().alias(&values).send().await.unwrap();
        service
            .search()
            .alias(values.as_slice())
            .send()
            .await
            .unwrap();

        // slice str
        let values = &["value1", "value2"];
        service.search().alias(values).send().await.unwrap();

        // hashset str
        let values = HashSet::from(["value1", "value2"]);
        service.search().alias(&values).send().await.unwrap();
        // hashset owned
        let values: HashSet<_> = values.iter().map(|x| x.to_string()).collect();
        service.search().alias(&values).send().await.unwrap();
        // hashset ref
        let values: HashSet<_> = values.iter().collect();
        service.search().alias(&values).send().await.unwrap();

        // IndexSet str
        let values = IndexSet::from(["value1", "value2"]);
        service.search().alias(&values).send().await.unwrap();
        // IndexSet owned
        let values: IndexSet<_> = values.iter().map(|x| x.to_string()).collect();
        service.search().alias(&values).send().await.unwrap();
        // IndexSet ref
        let values: IndexSet<_> = values.iter().collect();
        service.search().alias(&values).send().await.unwrap();
    }

    // ExistsOrValues<RangeOrValue<i64>> trait conversion testing
    #[tokio::test]
    async fn exists_or_values_range_i64() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();
        server.respond(200, path.join("search/ids.json")).await;

        // boolean
        service.search().blocks(true).send().await.unwrap();
        service.search().blocks(false).send().await.unwrap();

        // i64
        service.search().blocks(1).send().await.unwrap();

        // array
        service.search().blocks([1, 2]).send().await.unwrap();

        // vector
        let values = vec![1, 2];
        service.search().blocks(&values).send().await.unwrap();
        service
            .search()
            .blocks(values.as_slice())
            .send()
            .await
            .unwrap();

        // slice
        let values = &[1, 2];
        service.search().blocks(values).send().await.unwrap();

        // hashset
        let values = HashSet::from([1, 2]);
        service.search().blocks(&values).send().await.unwrap();

        // IndexSet str
        let values = IndexSet::from([1, 2]);
        service.search().blocks(&values).send().await.unwrap();
    }

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        server
            .respond(200, path.join("search/nonexistent.json"))
            .await;

        // values using all match operator variants
        let matches: Vec<_> = MatchOp::iter().map(|op| format!("{op} value")).collect();

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

        // alias
        service.search().alias(true).send().await.unwrap();
        service.search().alias(false).send().await.unwrap();
        service.search().alias("value").send().await.unwrap();
        service.search().alias(&matches).send().await.unwrap();

        // attachments
        service.search().attachments(true).send().await.unwrap();
        service.search().attachments(false).send().await.unwrap();
        service.search().attachments("value").send().await.unwrap();
        service.search().attachments(&matches).send().await.unwrap();

        // flags
        service.search().flags(true).send().await.unwrap();
        service.search().flags(false).send().await.unwrap();
        service.search().flags("value").send().await.unwrap();
        service.search().flags(&matches).send().await.unwrap();

        // groups
        service.search().groups(true).send().await.unwrap();
        service.search().groups(false).send().await.unwrap();
        service.search().groups("value").send().await.unwrap();
        service.search().groups(&matches).send().await.unwrap();

        // keywords
        service.search().keywords(true).send().await.unwrap();
        service.search().keywords(false).send().await.unwrap();
        service.search().keywords("value").send().await.unwrap();
        service.search().keywords(&matches).send().await.unwrap();

        // see_also
        service.search().see_also(true).send().await.unwrap();
        service.search().see_also(false).send().await.unwrap();
        service.search().see_also("value").send().await.unwrap();
        service.search().see_also(&matches).send().await.unwrap();

        // tags
        service.search().tags(true).send().await.unwrap();
        service.search().tags(false).send().await.unwrap();
        service.search().tags("value").send().await.unwrap();
        service.search().tags(&matches).send().await.unwrap();

        // whiteboard
        service.search().whiteboard(true).send().await.unwrap();
        service.search().whiteboard(false).send().await.unwrap();
        service.search().whiteboard("value").send().await.unwrap();
        service.search().whiteboard(&matches).send().await.unwrap();

        // url
        service.search().url(true).send().await.unwrap();
        service.search().url(false).send().await.unwrap();
        service.search().url("value").send().await.unwrap();
        service.search().url(&matches).send().await.unwrap();

        // change related combinators
        for field in StaticChangeField::iter() {
            // ever changed
            service.search().changed(&field).send().await.unwrap();

            // changed at a certain time
            for time in &times {
                service
                    .search()
                    .changed_at(&field, time.parse().unwrap())
                    .send()
                    .await
                    .unwrap();
            }

            // invalid equality operator usage
            for time in ["=2020", "!=2020-02-01", "=1d", "!=1w"] {
                assert!(service
                    .search()
                    .changed_at(&field, time.parse().unwrap())
                    .send()
                    .await
                    .is_err());
            }

            // changed by certain user(s)
            service
                .search()
                .changed_by(&field, ["user1", "user2"])
                .send()
                .await
                .unwrap();

            // changed from certain value
            service
                .search()
                .changed_from(&field, "value")
                .send()
                .await
                .unwrap();

            // changed to certain value
            service
                .search()
                .changed_to(&field, "value")
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

        // assignee
        service.search().assignee(["value"]).send().await.unwrap();

        // attacher
        service.search().attacher(["value"]).send().await.unwrap();

        // cc
        service.search().cc(true).send().await.unwrap();
        service.search().cc(false).send().await.unwrap();
        service.search().cc("value").send().await.unwrap();
        service.search().cc(&matches).send().await.unwrap();

        // commenter
        service.search().commenter(["value"]).send().await.unwrap();

        // flagger
        service.search().flagger(["value"]).send().await.unwrap();

        // qa
        service.search().qa(true).send().await.unwrap();
        service.search().qa(false).send().await.unwrap();
        service.search().qa("value").send().await.unwrap();
        service.search().qa(&matches).send().await.unwrap();

        // reporter
        service.search().reporter(["value"]).send().await.unwrap();

        // fields
        service
            .search()
            .fields([BugField::Id])
            .send()
            .await
            .unwrap();
        service
            .search()
            .fields([GroupField::All])
            .send()
            .await
            .unwrap();
        for field in FilterField::iter() {
            service.search().fields([field]).send().await.unwrap();
        }

        // blocks
        service.search().blocks(true).send().await.unwrap();
        service.search().blocks(false).send().await.unwrap();
        service.search().blocks(1).send().await.unwrap();
        service.search().blocks(-1).send().await.unwrap();
        service.search().blocks([1, -2]).send().await.unwrap();
        service.search().blocks(10..20).send().await.unwrap();
        service.search().blocks(10..=20).send().await.unwrap();
        service.search().blocks(..20).send().await.unwrap();
        service.search().blocks(..=20).send().await.unwrap();
        service.search().blocks(10..).send().await.unwrap();
        service.search().blocks(..).send().await.unwrap();

        // depends
        service.search().depends(true).send().await.unwrap();
        service.search().depends(false).send().await.unwrap();
        service.search().depends(1).send().await.unwrap();
        service.search().depends(-1).send().await.unwrap();
        service.search().depends([1, -2]).send().await.unwrap();
        service.search().depends(10..20).send().await.unwrap();
        service.search().depends(10..=20).send().await.unwrap();
        service.search().depends(..20).send().await.unwrap();
        service.search().depends(..=20).send().await.unwrap();
        service.search().depends(10..).send().await.unwrap();
        service.search().depends(..).send().await.unwrap();

        // ids
        service.search().ids(true).send().await.unwrap();
        service.search().ids(false).send().await.unwrap();
        service.search().ids(1).send().await.unwrap();
        service.search().ids(-1).send().await.unwrap();
        service.search().ids([1, -2]).send().await.unwrap();
        service.search().ids(10..20).send().await.unwrap();
        service.search().ids(10..=20).send().await.unwrap();
        service.search().ids(..20).send().await.unwrap();
        service.search().ids(..=20).send().await.unwrap();
        service.search().ids(10..).send().await.unwrap();
        service.search().ids(..).send().await.unwrap();
        for s in &id_ranges {
            let range: ExistsOrValues<RangeOrValue<i64>> = s.parse().unwrap();
            service.search().ids(range).send().await.unwrap();
        }

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

        service.search().limit(10).send().await.unwrap();
        service.search().offset(10).send().await.unwrap();
        service
            .search()
            .quicksearch("ALL @user OR reporter:user")
            .send()
            .await
            .unwrap();
    }
}
