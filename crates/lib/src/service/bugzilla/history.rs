use chrono::offset::Utc;
use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Event;
use crate::time::TimeDelta;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct HistoryRequest(Url);

impl HistoryRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        created: Option<&TimeDelta>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let [id, remaining_ids @ ..] = ids else {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        };

        let mut url = service.base().join(&format!("rest/bug/{id}/history"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in remaining_ids {
            url.query_pairs_mut().append_pair("ids", &id.to_string());
        }

        if let Some(interval) = created {
            let datetime = Utc::now() - interval.delta();
            let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
            url.query_pairs_mut().append_pair("new_since", &target);
        }

        Ok(Self(url))
    }
}

impl Request for HistoryRequest {
    type Output = Vec<Vec<Event>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let request = service.client().get(self.0);
        let response = service.send(request).await?;
        let mut data = service.parse_response(response).await?;
        let Value::Array(bugs) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid history data returned".to_string(),
            ));
        };

        let mut history = vec![];
        for mut bug in bugs {
            let data = bug["history"].take();
            history.push(serde_json::from_value(data)?);
        }

        Ok(history)
    }
}
