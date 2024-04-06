use serde_json::Value;

use crate::objects::bugzilla::Comment;
use crate::time::TimeDelta;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CommentRequest {
    url: url::Url,
    params: Option<CommentParams>,
}

impl CommentRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        params: Option<CommentParams>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let [id, remaining_ids @ ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut url = service.base().join(&format!("rest/bug/{id}/comment"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in remaining_ids {
            url.query_pairs_mut().append_pair("ids", &id.to_string());
        }

        if let Some(params) = params.as_ref() {
            if let Some(value) = params.created_after.as_ref() {
                url.query_pairs_mut()
                    .append_pair("new_since", &value.to_string());
            }
        }

        Ok(Self { url, params })
    }
}

impl Request for CommentRequest {
    type Output = Vec<Vec<Comment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.url).inject_auth(service, false)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        let serde_json::value::Value::Object(data) = data else {
            return Err(Error::InvalidValue(
                "invalid service response to comment request".to_string(),
            ));
        };

        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut comments = vec![];
        let params = self.params.unwrap_or_default();

        for (_id, mut data) in data {
            let Value::Array(data) = data["comments"].take() else {
                return Err(Error::InvalidValue(
                    "invalid service response to comment request".to_string(),
                ));
            };

            // deserialize and filter comments
            let mut bug_comments = vec![];
            for value in data {
                let comment: Comment = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidValue(format!("failed deserializing comment: {e}"))
                })?;
                if params.filter(&comment) {
                    bug_comments.push(comment);
                }
            }

            comments.push(bug_comments);
        }
        Ok(comments)
    }
}

/// Construct bug comment parameters.
#[derive(Debug, Default)]
pub struct CommentParams {
    attachment: Option<bool>,
    created_after: Option<TimeDelta>,
    creator: Option<String>,
}

impl CommentParams {
    pub fn new() -> Self {
        Self::default()
    }

    fn filter(&self, comment: &Comment) -> bool {
        if let Some(value) = self.attachment {
            if comment.attachment_id.is_some() != value {
                return false;
            }
        }

        if let Some(value) = self.creator.as_ref() {
            if !comment.creator.contains(value) {
                return false;
            }
        }

        true
    }

    pub fn attachment(&mut self, value: bool) {
        self.attachment = Some(value);
    }

    pub fn created_after(&mut self, interval: TimeDelta) {
        self.created_after = Some(interval);
    }

    pub fn creator<S>(&mut self, value: S)
    where
        S: Into<String>,
    {
        self.creator = Some(value.into());
    }
}
