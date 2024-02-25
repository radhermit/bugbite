use itertools::Itertools;
use url::Url;

use crate::objects::redmine::{Comment, Issue};
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct GetRequest {
    urls: Vec<Url>,
    comments: bool,
}

impl GetRequest {
    pub(super) fn new<S>(
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

        let mut urls = vec![];
        for id in ids {
            let mut url = service.config.web_base.join(&format!("issues/{id}.json"))?;
            // conditionally request additional data fields
            let mut fields = vec![];
            if comments {
                fields.push("journals");
            }
            if !fields.is_empty() {
                url.query_pairs_mut()
                    .append_pair("include", &fields.iter().join(","));
            }
            urls.push(url);
        }

        Ok(Self { urls, comments })
    }
}

impl Request for GetRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .urls
            .into_iter()
            .map(|u| service.client().get(u).send())
            .collect();

        let mut issues = vec![];
        for future in futures {
            let response = future.await?;
            let mut data = service.parse_response(response).await?;
            let mut data = data["issue"].take();
            let journals = data["journals"].take();
            let mut issue: Issue = serde_json::from_value(data)?;

            if self.comments {
                let mut count = 0;
                if let Some(text) = issue.description.take() {
                    issue.comments.push(Comment {
                        count,
                        text,
                        creator: issue.creator.clone().unwrap(),
                        created: issue.created.unwrap(),
                    });
                }

                // TODO: handle parsing changes within journal data
                if let serde_json::Value::Array(values) = journals {
                    for data in values {
                        let mut comment: Comment = serde_json::from_value(data)?;
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
