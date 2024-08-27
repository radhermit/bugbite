use serde_json::Value;

use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
}

impl<'a> Request<'a> {
    pub(super) fn new(service: &'a Service) -> Self {
        Self { service }
    }
}

impl RequestSend for Request<'_> {
    type Output = String;

    async fn send(self) -> crate::Result<Self::Output> {
        let url = self.service.config.base.join("rest/version")?;
        let request = self.service.client.get(url).auth_optional(self.service);

        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::String(version) = data["version"].take() else {
            return Err(Error::InvalidResponse(
                "invalid service response to version request".to_string(),
            ));
        };

        Ok(version)
    }
}
