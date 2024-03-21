use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::Request;

pub(crate) struct GetRequest(Vec<reqwest::Request>);

impl Request for GetRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
