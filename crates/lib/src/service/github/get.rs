use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::Request;

pub(crate) struct GetRequest(Vec<reqwest::Request>);

impl Request for GetRequest {
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
