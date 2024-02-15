use tracing::debug;

use crate::objects::bugzilla::Bug;
use crate::traits::{Request, WebService};
use crate::Error;

use super::attachments::AttachmentsRequest;
use super::comments::CommentsRequest;

type CombinedRequest = (
    reqwest::Request,
    Option<CommentsRequest>,
    Option<AttachmentsRequest>,
);

#[derive(Debug)]
pub(crate) struct GetRequest(Vec<CombinedRequest>);

impl GetRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        comments: bool,
        attachments: bool,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let mut requests = vec![];
        for id in ids {
            let url = service
                .base()
                .join(&format!("rest/bug/{id}"))
                .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;
            let req = service.client().get(url).build()?;
            let comment_req = if comments {
                Some(CommentsRequest::new(service, id)?)
            } else {
                None
            };
            let attachment_req = if attachments {
                Some(AttachmentsRequest::new(service, id)?)
            } else {
                None
            };
            requests.push((req, comment_req, attachment_req));
        }

        Ok(Self(requests))
    }
}

impl Request for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let mut futures = vec![];
        for (req, comment_req, attachment_req) in self.0 {
            futures.push((
                service.client().execute(req),
                comment_req.map(|r| r.send(service)),
                attachment_req.map(|r| r.send(service)),
            ));
        }

        let mut bugs = vec![];
        for (future, comments_future, attachments_future) in futures {
            let response = future.await?;
            let mut data = service.parse_response(response).await?;
            let data = data["bugs"][0].take();
            debug!("get request data: {data}");
            let mut bug: Bug = serde_json::from_value(data)?;
            if let Some(f) = comments_future {
                bug.comments = f.await?;
            }
            if let Some(f) = attachments_future {
                bug.attachments = f.await?;
            }
            bugs.push(bug);
        }

        Ok(bugs)
    }
}
