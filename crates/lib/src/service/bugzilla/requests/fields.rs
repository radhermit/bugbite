use serde::Deserialize;

use crate::service::bugzilla::Bugzilla;
use crate::service::bugzilla::objects::BugzillaField;
use crate::traits::{ParseResponse, RequestSend};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
}

impl Request {
    pub(crate) fn new(service: Bugzilla) -> Self {
        Self { service }
    }
}

#[derive(Deserialize, Debug)]
struct Response {
    fields: Vec<BugzillaField>,
}

impl RequestSend for Request {
    type Output = Vec<BugzillaField>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.service.config().base.join("rest/field/bug")?;
        let request = self.service.client().get(url);
        let response = request.send().await?;
        let data: Response = self.service.parse_response(response).await?;
        Ok(data.fields)
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
