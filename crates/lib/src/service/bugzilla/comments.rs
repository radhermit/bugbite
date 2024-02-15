use tracing::debug;

use crate::objects::bugzilla::Comment;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct CommentsRequest {
    id: String,
    req: reqwest::Request,
}

impl CommentsRequest {
    pub(super) fn new<S>(service: &super::Service, id: S) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let id = id.to_string();
        let url = service
            .base()
            .join(&format!("rest/bug/{id}/comment"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;

        Ok(Self {
            id,
            req: service.client().get(url).build()?,
        })
    }
}

impl Request for CommentsRequest {
    type Output = Vec<Comment>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"][&self.id]["comments"].take();
        debug!("comments request data: {data}");
        Ok(serde_json::from_value(data)?)
    }
}
