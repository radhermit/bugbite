use tracing::debug;

use crate::objects::bugzilla::Bug;
use crate::traits::{Request, WebService};
use crate::Error;

use super::attachments::AttachmentsRequest;
use super::comments::CommentsRequest;
use super::history::HistoryRequest;

type CombinedRequest = (
    reqwest::Request,
    Option<AttachmentsRequest>,
    Option<CommentsRequest>,
    Option<HistoryRequest>,
);

#[derive(Debug)]
pub(crate) struct GetRequest(Vec<CombinedRequest>);

impl GetRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        attachments: bool,
        comments: bool,
        history: bool,
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
            let attachments_req = if attachments {
                Some(
                    AttachmentsRequest::builder()
                        .bug_ids(&[id])
                        .build(service)?,
                )
            } else {
                None
            };
            let comments_req = if comments {
                Some(CommentsRequest::new(service, id)?)
            } else {
                None
            };
            let history_req = if history {
                Some(HistoryRequest::new(service, &[id], None)?)
            } else {
                None
            };
            requests.push((req, attachments_req, comments_req, history_req));
        }

        Ok(Self(requests))
    }
}

impl Request for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let mut futures = vec![];
        for (req, attachments_req, comments_req, history_req) in self.0 {
            futures.push((
                service.client().execute(req),
                attachments_req.map(|r| r.send(service)),
                comments_req.map(|r| r.send(service)),
                history_req.map(|r| r.send(service)),
            ));
        }

        let mut bugs = vec![];
        for (future, attachments, comments, history) in futures {
            let response = future.await?;
            let mut data = service.parse_response(response).await?;
            let data = data["bugs"][0].take();
            debug!("get request data: {data}");
            let mut bug: Bug = serde_json::from_value(data)?;
            if let Some(f) = attachments {
                bug.attachments = f.await?;
            }
            if let Some(f) = comments {
                bug.comments = f.await?;
            }
            if let Some(f) = history {
                bug.history = f.await?;
            }
            bugs.push(bug);
        }

        Ok(bugs)
    }
}
