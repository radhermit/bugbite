use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::RequestSend;

pub struct GetRequest(Vec<reqwest::Request>);

impl RequestSend for GetRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
