use crate::Error;
use crate::objects::bugzilla::BugzillaField;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{RequestSend, WebService};

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
    type Output = Vec<BugzillaField>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config().base.join("rest/field/bug")?;
        let request = self.service.client().get(url);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        serde_json::from_value(data["fields"].take())
            .map_err(|e| Error::InvalidResponse(format!("failed deserializing fields: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        server.respond(200, path.join("fields/gentoo.json")).await;
        let fields = service.fields().send().await.unwrap();
        assert!(!fields.is_empty());
    }
}
