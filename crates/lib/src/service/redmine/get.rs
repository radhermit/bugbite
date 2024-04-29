use itertools::Itertools;
use reqwest::StatusCode;
use url::Url;

use crate::objects::redmine::{Comment, Issue};
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct GetRequest {
    ids: Vec<String>,
    urls: Vec<Url>,
    comments: bool,
}

impl GetRequest {
    pub(crate) fn new<S>(
        service: &super::Service,
        ids: &[S],
        _attachments: bool,
        comments: bool,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        if ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut request_ids = vec![];
        let mut urls = vec![];

        // conditionally request additional data fields
        let mut fields = vec![];
        if comments {
            fields.push("journals");
        }

        for id in ids {
            let mut url = service.config.web_base.join(&format!("issues/{id}.json"))?;
            if !fields.is_empty() {
                url.query_pairs_mut()
                    .append_pair("include", &fields.iter().join(","));
            }
            request_ids.push(id.to_string());
            urls.push(url);
        }

        Ok(Self {
            ids: request_ids,
            urls,
            comments,
        })
    }
}

impl RequestSend for GetRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .urls
            .into_iter()
            .map(|u| service.client.get(u))
            .map(|r| r.auth_optional(service).map(|r| r.send()))
            .try_collect()?;

        let mut issues = vec![];
        for (future, id) in futures.into_iter().zip(self.ids.into_iter()) {
            let response = future.await?;
            let mut data = service
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

            if self.comments {
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
