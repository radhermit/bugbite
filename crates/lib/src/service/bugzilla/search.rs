use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::{fmt, fs};

use async_stream::try_stream;
use camino::Utf8Path;
use futures::stream::Stream;
use indexmap::IndexSet;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use strum::{Display, EnumIter, EnumString, VariantNames};
use url::Url;

use crate::args::ExistsOrValues;
use crate::objects::bugzilla::Bug;
use crate::objects::{Range, RangeOp, RangeOrValue};
use crate::query::{Order, Query};
use crate::service::bugzilla::Service;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{Api, InjectAuth, RequestMerge, RequestSend, WebService};
use crate::utils::or;
use crate::Error;

use super::{BugField, FilterField};

#[derive(Serialize, Debug, Clone)]
pub struct Request<'a> {
    #[serde(skip)]
    service: &'a Service,
    #[serde(flatten)]
    pub params: Parameters,
}

impl RequestMerge<&Utf8Path> for Request<'_> {
    fn merge(&mut self, path: &Utf8Path) -> crate::Result<()> {
        let params = Parameters::from_path(path)?;
        self.params.merge(params);
        Ok(())
    }
}

impl<T: Into<Parameters>> RequestMerge<T> for Request<'_> {
    fn merge(&mut self, value: T) -> crate::Result<()> {
        self.params.merge(value);
        Ok(())
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<Bug>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let mut url = self.service.config.base.join("rest/bug")?;
        let params = self.encode()?;
        url.query_pairs_mut().extend_pairs(&params.query);
        let request = self.service.client.get(url).auth_optional(self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let mut bugs = vec![];
        if let serde_json::Value::Array(values) = data["bugs"].take() {
            for mut value in values {
                let custom_fields = self.service.deserialize_custom_fields(&mut value);
                let mut bug: Bug = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidResponse(format!("failed deserializing bug: {e}"))
                })?;
                bug.custom_fields = custom_fields;
                bugs.push(bug);
            }
        }
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

    // TODO: submit multiple requests at once?
    pub async fn stream(&self) -> impl Stream<Item = crate::Result<Bug>> + '_ {
        try_stream! {
            // TODO: pull max from service config
            let limit = self.params.limit.unwrap_or(10000);
            let mut offset = self.params.offset.unwrap_or_default();
            let mut req = self.clone().limit(limit);

            loop {
                req.params.offset = Some(offset);
                let items = req.send().await?;

                // no more items exist
                if items.is_empty() {
                    break;
                }

                let mut count = 0;
                for bug in items {
                    yield bug;
                    count += 1;
                }

                // no additional items exist
                if count < limit {
                    break;
                }

                offset += limit;
            }
        }
    }

    fn encode(&self) -> crate::Result<QueryBuilder> {
        let mut query = QueryBuilder::new(self.service);

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
                        ExistsOrValues::Exists(true) => query.exists(ExistsField::Alias, true),
                        ExistsOrValues::Exists(false) => {
                            query.not(|query| query.exists(ExistsField::Alias, true))
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
                query.changed(fields.iter().map(|f| (f, interval)));
            }
        }

        if let Some(values) = &self.params.changed_by {
            for (fields, users) in values {
                query.changed_by(fields.iter().map(|f| (f, users)));
            }
        }

        if let Some(values) = &self.params.changed_from {
            query.changed_from(values);
        }

        if let Some(values) = &self.params.changed_to {
            query.changed_to(values);
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
                            query.and(|query| values.iter().for_each(|x| query.blocks(*x)))
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
                            query.and(|query| values.iter().for_each(|x| query.depends(*x)))
                        }
                    }
                }
            });
        }

        if let Some(values) = &self.params.ids {
            query.or(|query| values.iter().for_each(|x| query.id(x)));
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
            query.changed([(&ChangeField::Status, value)]);
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
    pub fn search_url(self) -> crate::Result<Url> {
        let mut url = self.service.config.base.join("buglist.cgi")?;
        let params = self.encode()?;
        url.query_pairs_mut().extend_pairs(&params.query);
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

    pub fn changed<F: fmt::Display>(mut self, field: F) -> Self {
        self.params
            .changed
            .get_or_insert_with(Default::default)
            .push((vec![field.to_string()], "<now".parse().unwrap()));
        self
    }

    pub fn changed_at<F: fmt::Display>(
        mut self,
        field: F,
        value: RangeOrValue<TimeDeltaOrStatic>,
    ) -> Self {
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
        T: Into<ExistsOrValues<i64>>,
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
        T: Into<ExistsOrValues<i64>>,
    {
        // TODO: move to get_or_insert_default() when it is stable
        self.params
            .depends
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

    pub fn limit(mut self, value: u64) -> Self {
        self.params.limit = Some(value);
        self
    }

    pub fn offset(mut self, value: u64) -> Self {
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
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub order: Option<Vec<Order<OrderField>>>,

    pub created: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub updated: Option<RangeOrValue<TimeDeltaOrStatic>>,
    pub closed: Option<RangeOrValue<TimeDeltaOrStatic>>,

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
    pub custom_fields: Option<Vec<(String, ExistsOrValues<Match>)>>,
}

impl Parameters {
    /// Load parameters in TOML format from a file.
    fn from_path(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::InvalidValue(format!("failed loading template: {path}: {e}")))?;
        toml::from_str(&data)
            .map_err(|e| Error::InvalidValue(format!("failed parsing template: {path}: {e}")))
    }

    /// Override parameters using the provided value if it exists.
    fn merge<T: Into<Self>>(&mut self, other: T) {
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
        or!(self.offset, other.offset);
        or!(self.order, other.order);

        or!(self.created, other.created);
        or!(self.updated, other.updated);
        or!(self.closed, other.closed);

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
}

/// Construct a search query.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug)]
struct QueryBuilder<'a> {
    service: &'a Service,
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
    fn new(service: &'a Service) -> Self {
        Self {
            service,
            query: Default::default(),
            advanced_count: Default::default(),
        }
    }
}

/// Advanced field matching operators.
#[derive(Display, EnumIter, EnumString, Debug, PartialEq, Eq, Clone, Copy)]
enum MatchOp {
    /// Contains case-sensitive substring.
    #[strum(serialize = "=~")]
    CaseSubstring,
    /// Contains substring.
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
    op: MatchOp,
    value: String,
}

impl Match {
    /// Substitute user alias for matching value.
    fn replace_user_alias(&self, service: &Service) -> Self {
        Self {
            op: self.op,
            value: service.replace_user_alias(&self.value).to_string(),
        }
    }
}

impl Api for Match {
    fn api(&self) -> String {
        self.value.to_string()
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.op, self.value)
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
            (op, value.into())
        } else {
            (MatchOp::Substring, s.into())
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

impl From<i64> for ExistsOrValues<i64> {
    fn from(value: i64) -> Self {
        ExistsOrValues::Values(vec![value])
    }
}

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
        impl From<$x> for ExistsOrValues<i64> {
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

impl<const N: usize> From<&[i64; N]> for ExistsOrValues<i64> {
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

impl<const N: usize> From<[i64; N]> for ExistsOrValues<i64> {
    fn from(values: [i64; N]) -> Self {
        ExistsOrValues::Values(values.into_iter().map(Into::into).collect())
    }
}

impl QueryBuilder<'_> {
    fn id(&mut self, value: &RangeOrValue<i64>) {
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
        self.advanced_field("alias", value.op, value);
    }

    fn assignee(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("assigned_to", value.op, value);
    }

    /// Search for attachments with matching descriptions or filenames.
    fn attachments(&mut self, values: &[Match]) {
        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "OP");
        self.insert(format!("j{num}"), "OR");

        for value in values {
            self.advanced_field("attachments.description", value.op, value);
            self.advanced_field("attachments.filename", value.op, value);
        }

        self.advanced_count += 1;
        let num = self.advanced_count;
        self.insert(format!("f{num}"), "CP");
    }

    fn attachment_description(&mut self, value: &Match) {
        self.advanced_field("attachments.description", value.op, value);
    }

    fn attachment_filename(&mut self, value: &Match) {
        self.advanced_field("attachments.filename", value.op, value);
    }

    fn attachment_mime(&mut self, value: &Match) {
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

    fn comment(&mut self, value: &Match) {
        self.advanced_field("longdesc", value.op, value);
    }

    fn comment_is_private(&mut self, value: bool) {
        self.boolean("longdescs.isprivate", value)
    }

    fn comment_tag(&mut self, value: &Match) {
        self.advanced_field("comment_tag", value.op, value);
    }

    fn qa(&mut self, value: &Match) {
        self.advanced_field("qa_contact", value.op, value);
    }

    fn reporter(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("reporter", value.op, value);
    }

    fn resolution(&mut self, value: &Match) {
        self.advanced_field("resolution", value.op, value);
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
        self.advanced_field("attachments.submitter", value.op, value);
    }

    fn commenter(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("commenter", value.op, value);
    }

    fn flagger(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
        self.advanced_field("setters.login_name", value.op, value);
    }

    fn url(&mut self, value: &Match) {
        self.advanced_field("bug_file_loc", value.op, value);
    }

    fn changed<'a, F, I>(&mut self, values: I)
    where
        F: Api,
        I: IntoIterator<Item = (F, &'a RangeOrValue<TimeDeltaOrStatic>)>,
    {
        for (field, target) in values {
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

    fn changed_by<F, I, J, S>(&mut self, values: I)
    where
        F: Api,
        I: IntoIterator<Item = (F, J)>,
        J: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for (field, users) in values {
            for user in users {
                let user = self.service.replace_user_alias(user.as_ref());
                self.advanced_field(&field, "changedby", user);
            }
        }
    }

    fn changed_from<'a, F, I, S>(&mut self, values: I)
    where
        F: Api + 'a,
        I: IntoIterator<Item = &'a (F, S)>,
        S: Api + 'a,
    {
        for (field, value) in values {
            self.advanced_field(field, "changedfrom", value);
        }
    }

    fn changed_to<'a, F, I, S>(&mut self, values: I)
    where
        F: Api + 'a,
        I: IntoIterator<Item = &'a (F, S)>,
        S: Api + 'a,
    {
        for (field, value) in values {
            self.advanced_field(field, "changedto", value);
        }
    }

    fn custom_field<F: Api>(&mut self, name: F, value: &Match) {
        self.advanced_field(name, value.op, value);
    }

    fn priority(&mut self, value: &Match) {
        self.advanced_field("priority", value.op, value);
    }

    fn severity(&mut self, value: &Match) {
        self.advanced_field("bug_severity", value.op, value);
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
        self.advanced_field("version", value.op, value);
    }

    fn component(&mut self, value: &Match) {
        self.advanced_field("component", value.op, value);
    }

    fn product(&mut self, value: &Match) {
        self.advanced_field("product", value.op, value);
    }

    fn platform(&mut self, value: &Match) {
        self.advanced_field("platform", value.op, value);
    }

    fn os(&mut self, value: &Match) {
        self.advanced_field("op_sys", value.op, value);
    }

    fn see_also(&mut self, value: &Match) {
        self.advanced_field("see_also", value.op, value);
    }

    fn summary(&mut self, value: &Match) {
        self.advanced_field("short_desc", value.op, value);
    }

    fn tags(&mut self, value: &Match) {
        self.advanced_field("tag", value.op, value);
    }

    fn target(&mut self, value: &Match) {
        self.advanced_field("target_milestone", value.op, value);
    }

    fn whiteboard(&mut self, value: &Match) {
        self.advanced_field("whiteboard", value.op, value);
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

    fn flags(&mut self, value: &Match) {
        self.advanced_field("flagtypes.name", value.op, value)
    }

    fn groups(&mut self, value: &Match) {
        self.advanced_field("bug_group", value.op, value)
    }

    fn keywords(&mut self, value: &Match) {
        self.advanced_field("keywords", value.op, value)
    }

    fn cc(&mut self, value: &Match) {
        let value = value.replace_user_alias(self.service);
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

    fn not<F: FnOnce(&mut Self)>(&mut self, func: F) {
        func(self);
        let num = self.advanced_count;
        self.insert(format!("n{num}"), "1");
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

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use crate::service::bugzilla::{Config, GroupField};
    use crate::test::*;

    use super::*;

    // From<ExistsOrValues<Match>> trait conversion testing
    #[tokio::test]
    async fn exists_or_values_match() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();
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

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        server
            .respond(200, path.join("search/nonexistent.json"))
            .await;

        // values using all match operator variants
        let matches: Vec<_> = MatchOp::iter().map(|op| format!("{op} value")).collect();

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
        for field in ChangeField::iter() {
            // ever changed
            service.search().changed(field).send().await.unwrap();

            // changed at a certain time
            for time in [
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
            ] {
                service
                    .search()
                    .changed_at(field, time.parse().unwrap())
                    .send()
                    .await
                    .unwrap();
            }

            // changed by certain user(s)
            service
                .search()
                .changed_by(field, ["user1", "user2"])
                .send()
                .await
                .unwrap();

            // changed from certain value
            service
                .search()
                .changed_from(field, "value")
                .send()
                .await
                .unwrap();

            // changed to certain value
            service
                .search()
                .changed_to(field, "value")
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

        // depends
        service.search().depends(true).send().await.unwrap();
        service.search().depends(false).send().await.unwrap();
        service.search().depends(1).send().await.unwrap();
        service.search().depends(-1).send().await.unwrap();
        service.search().depends([1, -2]).send().await.unwrap();
    }
}
