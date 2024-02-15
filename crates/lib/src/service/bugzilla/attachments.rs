use tracing::debug;

use crate::objects::bugzilla::Attachment;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct AttachmentsRequest {
    id: String,
    req: reqwest::Request,
}

impl AttachmentsRequest {
    pub(super) fn new<S>(service: &super::Service, id: S) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let id = id.to_string();
        let url = service
            .base()
            .join(&format!("rest/bug/{id}/attachment"))
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;

        Ok(Self {
            id,
            req: service.client().get(url).build()?,
        })
    }
}

impl Request for AttachmentsRequest {
    type Output = Vec<Attachment>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
        let mut data = service.parse_response(response).await?;
        let data = data["bugs"][&self.id].take();
        debug!("attachments request data: {data}");
        Ok(serde_json::from_value(data)?)
    }
}
