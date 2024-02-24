use itertools::Itertools;
use url::Url;

use crate::objects::redmine::{Comment, Issue};
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct GetRequest {
    url: Url,
}

impl GetRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        _attachments: bool,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        if ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut url = service.base().join("issues.json")?;
        url.query_pairs_mut()
            .append_pair("issue_id", &ids.iter().join(","));
        // force closed issues to be returned
        url.query_pairs_mut().append_pair("status_id", "*");

        Ok(Self { url })
    }
}

impl Request for GetRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().get(self.url).send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["issues"].take();
        let mut issues: Vec<Issue> = serde_json::from_value(data)?;
        for issue in &mut issues {
            issue.comments.push(Comment {
                count: 0,
                text: issue.description.take().unwrap(),
                creator: issue.creator.clone().unwrap(),
                created: issue.created.unwrap(),
            });
        }

        Ok(issues)
    }
}
