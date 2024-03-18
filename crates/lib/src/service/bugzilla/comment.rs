use chrono::offset::Utc;

use crate::objects::bugzilla::Comment;
use crate::time::TimeDelta;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CommentRequest(url::Url);

impl CommentRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        created: Option<&TimeDelta>,
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

        if let Some(interval) = created {
            let datetime = Utc::now() - interval.delta();
            let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
            url.query_pairs_mut().append_pair("new_since", &target);
        }

        Ok(Self(url))
    }
}

impl Request for CommentRequest {
    type Output = Vec<Vec<Comment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.0).inject_auth(service, false)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"].take();
        let serde_json::value::Value::Object(data) = data else {
            panic!("invalid bugzilla comment response");
        };
        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut comments = vec![];
        for (_id, mut data) in data {
            let data = data["comments"].take();
            comments.push(serde_json::from_value(data)?);
        }
        Ok(comments)
    }
}
