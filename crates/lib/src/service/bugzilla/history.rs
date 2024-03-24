use chrono::offset::Utc;
use serde_json::Value;

use crate::objects::bugzilla::Event;
use crate::time::TimeDelta;
use crate::traits::{InjectAuth, Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct HistoryRequest<'a> {
    url: url::Url,
    service: &'a super::Service,
}

impl<'a> HistoryRequest<'a> {
    pub(super) fn new<S>(
        service: &'a super::Service,
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

        Ok(Self { url, service })
    }
}

impl Request for HistoryRequest<'_> {
    type Output = Vec<Vec<Event>>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url)
            .inject_auth(self.service, false)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(bugs) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to history request".to_string(),
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
