use chrono::offset::Utc;
use itertools::Itertools;
use tracing::debug;
use url::Url;

use crate::objects::bugzilla::Comment;
use crate::time::TimeDelta;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CommentsRequest {
    ids: Vec<String>,
    req: reqwest::Request,
}

impl CommentsRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        created: Option<TimeDelta>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let mut params = vec![];
        let mut url = match ids {
            [id, ids @ ..] => {
                // Note that multiple request support is missing from upstream's REST API
                // documentation, but exists in older RPC-based docs.
                if !ids.is_empty() {
                    params.push(("ids".to_string(), ids.iter().join(",")));
                }
                service.base().join(&format!("/rest/bug/{id}/comment"))?
            }
            _ => return Err(Error::InvalidValue("invalid comments ID state".to_string())),
        };

        if let Some(interval) = created {
            let datetime = Utc::now() - interval.delta();
            let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
            params.push(("new_since".to_string(), target));
        }

        if !params.is_empty() {
            url = Url::parse_with_params(url.as_str(), params)?;
        }

        debug!("comments request url: {url}");
        Ok(Self {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            req: service.client().get(url).build()?,
        })
    }
}

impl Request for CommentsRequest {
    type Output = Vec<Vec<Comment>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
        let mut data = service.parse_response(response).await?;
        debug!("comments request data: {data}");
        let mut data = data["bugs"].take();
        let mut comments = vec![];
        for id in self.ids {
            let data = data[&id]["comments"].take();
            comments.push(serde_json::from_value(data)?);
        }
        Ok(comments)
    }
}
