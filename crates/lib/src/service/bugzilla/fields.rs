use crate::objects::bugzilla::BugzillaField;
use crate::service::bugzilla::Service;
use crate::traits::{RequestSend, WebService};
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
    type Output = Vec<BugzillaField>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config.base.join("rest/field/bug")?;
        let request = self.service.client.get(url);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        serde_json::from_value(data["fields"].take())
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing fields: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        server.respond(200, path.join("fields/gentoo.json")).await;
        let fields = service.fields().send().await.unwrap();
        assert!(!fields.is_empty());
    }
}
