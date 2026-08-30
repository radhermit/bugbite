use serde::Deserialize;

use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
}

impl Request {
    pub(super) fn new(service: Bugzilla) -> Self {
        Self { service }
    }
}

#[derive(Deserialize, Debug)]
struct Response {
    version: String,
}

impl RequestSend for Request {
    type Output = String;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config().base.join("rest/version")?;
        let request = self.service.client().get(url).auth_optional(&self.service);
        let response = request.send().await?;
        let data: Response = self.service.parse_response(response).await?;
        Ok(data.version)
    }
}
