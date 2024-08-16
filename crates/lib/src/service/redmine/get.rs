use indexmap::IndexSet;
use itertools::Itertools;
use reqwest::StatusCode;
use strum::Display;
use url::Url;

use crate::objects::redmine::{Comment, Issue};
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a super::Service,
    ids: Vec<String>,
    fields: IndexSet<Field>,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I, S>(service: &'a super::Service, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service,
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            fields: Default::default(),
        }
    }

    fn urls(&self) -> crate::Result<Vec<Url>> {
        if self.ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        }

        let mut urls = vec![];
        for id in &self.ids {
            let mut url = self
                .service
                .config
                .web_base
                .join(&format!("issues/{id}.json"))?;
            if !self.fields.is_empty() {
                url.query_pairs_mut()
                    .append_pair("include", &self.fields.iter().join(","));
            }
            urls.push(url);
        }

        Ok(urls)
    }

    /// Enable or disable fetching attachments.
    pub fn attachments(mut self, fetch: bool) -> Self {
        if fetch {
            self.fields.insert(Field::Attachments);
        }
        self
    }

    /// Enable or disable fetching comments.
    pub fn comments(mut self, fetch: bool) -> Self {
        if fetch {
            self.fields.insert(Field::Journals);
        }
        self
    }
}

/// Bug fields composed of value arrays.
#[derive(Display, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
enum Field {
    Attachments,
    Journals,
}

impl RequestSend for Request<'_> {
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .urls()?
            .into_iter()
            .map(|u| self.service.client.get(u))
            .map(|r| r.auth_optional(self.service).map(|r| r.send()))
            .try_collect()?;

        let mut issues = vec![];
        for (future, id) in futures.into_iter().zip(self.ids.into_iter()) {
            let response = future.await?;
            let mut data = self
                .service
                .parse_response(response)
                .await
                .map_err(|e| match e {
                    Error::Request(e) if e.status() == Some(StatusCode::NOT_FOUND) => {
                        Error::Redmine(format!("nonexistent issue: {id}"))
                    }
                    _ => e,
                })?;
            let mut data = data["issue"].take();
            let journals = data["journals"].take();
            let mut issue: Issue = serde_json::from_value(data)
                .map_err(|e| Error::InvalidValue(format!("failed deserializing issue: {e}")))?;

            if self.fields.contains(&Field::Journals) {
                let mut count = 0;
                // treat description as a comment
                if let Some(text) = issue.description.take() {
                    issue.comments.push(Comment {
                        count,
                        text,
                        user: issue.author.clone().unwrap(),
                        created: issue.created.unwrap(),
                    });
                }

                // TODO: handle parsing changes within journal data
                if let serde_json::Value::Array(values) = journals {
                    for data in values {
                        let mut comment: Comment = serde_json::from_value(data).map_err(|e| {
                            Error::InvalidValue(format!("failed deserializing comment: {e}"))
                        })?;
                        if !comment.text.is_empty() {
                            count += 1;
                            comment.count = count;
                            issue.comments.push(comment);
                        }
                    }
                }
            }

            issues.push(issue);
        }

        Ok(issues)
    }
}
