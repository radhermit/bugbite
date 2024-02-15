use crate::objects::github::Issue;
use crate::traits::Request;

pub(crate) struct SearchRequest(reqwest::Request);

impl Request for SearchRequest {
    type Output = Vec<Issue>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        todo!()
    }
}
