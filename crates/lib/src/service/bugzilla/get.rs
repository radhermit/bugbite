use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Bug;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

use super::attachment::AttachmentRequest;
use super::comment::CommentRequest;
use super::history::HistoryRequest;

#[derive(Debug)]
pub(crate) struct GetRequest<'a> {
    url: Url,
    attachments: Option<AttachmentRequest<'a>>,
    comments: Option<CommentRequest<'a>>,
    history: Option<HistoryRequest<'a>>,
    service: &'a super::Service,
}

impl<'a> GetRequest<'a> {
    pub(super) fn new<S>(
        service: &'a super::Service,
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

        let mut url = service.base().join(&format!("rest/bug/{id}"))?;

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
            Some(service.attachment_request(ids, true, false)?)
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
            service,
        })
    }
}

impl Request for GetRequest<'_> {
    type Output = Vec<Bug>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url)
            .inject_auth(self.service, false)?;
        let (bugs, attachments, comments, history) = (
            request.send(),
            self.attachments.map(|r| r.send()),
            self.comments.map(|r| r.send()),
            self.history.map(|r| r.send()),
        );

        let response = bugs.await?;
        let mut data = self.service.parse_response(response).await?;
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
