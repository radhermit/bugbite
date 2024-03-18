use std::num::NonZeroU64;

use url::Url;

use crate::objects::bugzilla::Bug;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

use super::attachment::AttachmentRequest;
use super::comment::CommentRequest;
use super::history::HistoryRequest;

#[derive(Debug)]
pub(crate) struct GetRequest {
    url: Url,
    attachments: Option<AttachmentRequest>,
    comments: Option<CommentRequest>,
    history: Option<HistoryRequest>,
}

impl GetRequest {
    pub(super) fn new(
        service: &super::Service,
        ids: &[NonZeroU64],
        attachments: bool,
        comments: bool,
        history: bool,
    ) -> crate::Result<Self> {
        if ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut url = service.base().join("rest/bug")?;

        for id in ids {
            url.query_pairs_mut().append_pair("id", &id.to_string());
        }

        // drop useless token that is injected for authenticated requests
        url.query_pairs_mut()
            .append_pair("exclude_fields", "update_token");

        let attachments = if attachments {
            Some(service.item_attachment_request(ids, false)?)
        } else {
            None
        };
        let comments = if comments {
            Some(CommentRequest::new(service, ids, None)?)
        } else {
            None
        };
        let history = if history {
            Some(HistoryRequest::new(service, ids, None)?)
        } else {
            None
        };

        Ok(Self {
            url,
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
        let request = service.client().get(self.url).inject_auth(service, false)?;
        let (bugs, attachments, comments, history) = (
            request.send(),
            self.attachments.map(|r| r.send(service)),
            self.comments.map(|r| r.send(service)),
            self.history.map(|r| r.send(service)),
        );

        let response = bugs.await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
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
