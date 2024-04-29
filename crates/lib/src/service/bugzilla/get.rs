use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Bug;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

use super::attachment::get::AttachmentGetRequest;
use super::comment::CommentRequest;
use super::history::HistoryRequest;

#[derive(Debug)]
pub struct GetRequest {
    url: Url,
    attachments: Option<AttachmentGetRequest>,
    comments: Option<CommentRequest>,
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
        let [id, remaining_ids @ ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut url = service.config.base.join(&format!("rest/bug/{id}"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in remaining_ids {
            url.query_pairs_mut().append_pair("ids", &id.to_string());
        }

        // include personal tags
        url.query_pairs_mut()
            .append_pair("include_fields", "_default,tags");

        // drop useless token that is injected for authenticated requests
        url.query_pairs_mut()
            .append_pair("exclude_fields", "update_token");

        let attachments = if attachments {
            Some(service.attachment_get(ids, true, false)?)
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

impl RequestSend for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client.get(self.url).auth_optional(service)?;
        let (bugs, attachments, comments, history) = (
            request.send(),
            self.attachments.map(|r| r.send(service)),
            self.comments.map(|r| r.send(service)),
            self.history.map(|r| r.send(service)),
        );

        let response = bugs.await?;
        let mut data = service.parse_response(response).await?;
        let Value::Array(data) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to get request".to_string(),
            ));
        };

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

        let mut bugs = vec![];
        for value in data {
            let mut bug: Bug = serde_json::from_value(value)
                .map_err(|e| Error::InvalidValue(format!("failed deserializing bug: {e}")))?;
            bug.attachments = attachments.next().unwrap_or_default();
            bug.comments = comments.next().unwrap_or_default();
            bug.history = history.next().unwrap_or_default();
            bugs.push(bug);
        }

        Ok(bugs)
    }
}
