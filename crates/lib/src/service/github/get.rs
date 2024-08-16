use tracing::debug;

use crate::objects::github::Issue;
use crate::traits::RequestSend;

#[derive(Debug)]
pub struct Request(Vec<reqwest::Request>);

impl RequestSend for Request {
    type Output = Vec<Issue>;

    async fn send(self) -> crate::Result<Self::Output> {
        debug!("{:?}", self.0);
        todo!()
    }
}
