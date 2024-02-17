use itertools::Itertools;
use tracing::debug;

use crate::objects::bugzilla::Bug;
use crate::traits::{Request, WebService};

use super::attachments::AttachmentsRequest;
use super::comments::CommentsRequest;
use super::history::HistoryRequest;

#[derive(Debug)]
pub(crate) struct GetRequest {
    request: reqwest::Request,
    attachments: Option<AttachmentsRequest>,
    comments: Option<CommentsRequest>,
    history: Option<HistoryRequest>,
}

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
        let url = service
            .base()
            .join(&format!("/rest/bug?id={}", ids.iter().join(",")))?;
        let request = service.client().get(url).build()?;

        let attachments = if attachments {
            Some(service.item_attachments_request(ids, false)?)
        } else {
            None
        };
        let comments = if comments {
            Some(CommentsRequest::new(service, ids, None)?)
        } else {
            None
        };
        let history = if history {
            Some(HistoryRequest::new(service, ids, None)?)
        } else {
            None
        };

        Ok(Self {
            request,
            attachments,
            comments,
            history,
        })
    }
}

impl Request for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let (bugs, attachments, comments, history) = (
            service.client().execute(self.request),
            self.attachments.map(|r| r.send(service)),
            self.comments.map(|r| r.send(service)),
            self.history.map(|r| r.send(service)),
        );

        let response = bugs.await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        debug!("get request data: {data}");
        let mut bugs: Vec<Bug> = serde_json::from_value(data)?;

        let mut attachments = match attachments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut comments = match comments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut history = match history {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };

        for bug in &mut bugs {
            bug.attachments = attachments.next().unwrap_or_default();
            bug.comments = comments.next().unwrap_or_default();
            bug.history = history.next().unwrap_or_default();
        }

        Ok(bugs)
    }
}
