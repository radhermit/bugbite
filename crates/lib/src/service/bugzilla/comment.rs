use std::num::NonZeroU64;

use chrono::offset::Utc;
use url::Url;

use crate::objects::bugzilla::Comment;
use crate::time::TimeDelta;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CommentRequest {
    ids: Vec<String>,
    url: Url,
}

impl CommentRequest {
    pub(super) fn new(
        service: &super::Service,
        ids: &[NonZeroU64],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Self> {
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

        Ok(Self {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            url,
        })
    }
}

impl Request for CommentRequest {
    type Output = Vec<Vec<Comment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.url).inject_auth(service, false)?;
        let response = request.send().await?;
        let mut data = service.parse_response(response).await?;
        let mut data = data["bugs"].take();
        let mut comments = vec![];
        for id in self.ids {
            let data = data[&id]["comments"].take();
            comments.push(serde_json::from_value(data)?);
        }
        Ok(comments)
    }
}
