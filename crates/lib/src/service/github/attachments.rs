use crate::objects::github::Attachment;
use crate::traits::Request;

pub(crate) struct AttachmentsRequest(Vec<reqwest::Request>);

impl Request for AttachmentsRequest {
    type Output = Vec<Attachment>;
    type Service = super::Service;

    async fn send(self, _service: &Self::Service) -> crate::Result<Self::Output> {
        todo!()
    }
}
