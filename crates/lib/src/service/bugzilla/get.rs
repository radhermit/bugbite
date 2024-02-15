use tracing::debug;

use crate::objects::bugzilla::Bug;
use crate::traits::{Request, WebService};
use crate::Error;

pub(crate) struct GetRequest(Vec<reqwest::Request>);

impl Request for GetRequest {
    type Output = Vec<Bug>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let mut futures = vec![];
        for req in self.0 {
            futures.push(service.client().execute(req));
        }

        let mut bugs = vec![];
        for future in futures {
            let response = future.await?;
            let mut data = service.parse_response(response).await?;
            let data = data["bugs"][0].take();
            debug!("get request data: {data}");
            bugs.push(serde_json::from_value(data)?);
        }

        Ok(bugs)
    }
}

impl GetRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        _comments: bool,
        _attachments: bool,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let mut requests = vec![];
        for id in ids {
            let url = service
                .base()
                .join(&format!("rest/bug/{id}"))
                .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;
            requests.push(service.client().get(url).build()?);
        }
        Ok(Self(requests))
    }
}
