use serde_json::Value;

use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
}

impl Request {
    pub(super) fn new(service: &Bugzilla) -> Self {
        Self {
            service: service.clone(),
        }
    }
}

impl RequestSend for Request {
    type Output = String;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config.base.join("rest/version")?;
        let request = self.service.client.get(url).auth_optional(&self.service);

        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::String(version) = data["version"].take() else {
            return Err(Error::InvalidResponse("version request".to_string()));
        };

        Ok(version)
    }
}
