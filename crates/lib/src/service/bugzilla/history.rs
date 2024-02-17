use chrono::offset::Utc;
use itertools::Itertools;
use serde_json::Value;
use tracing::debug;
use url::Url;

use crate::objects::bugzilla::Event;
use crate::time::TimeDelta;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct HistoryRequest(reqwest::Request);

impl HistoryRequest {
    pub(super) fn new<S>(
        service: &super::Service,
        ids: &[S],
        created: Option<TimeDelta>,
    ) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let mut params = vec![];
        let mut url = match ids {
            [id, ids @ ..] => {
                // Note that multiple request support is missing from upstream's REST API
                // documentation, but exists in older RPC-based docs.
                if !ids.is_empty() {
                    params.push(("ids".to_string(), ids.iter().join(",")));
                }
                service.base().join(&format!("/rest/bug/{id}/history"))?
            }
            _ => return Err(Error::InvalidValue("invalid history ID state".to_string())),
        };

        if let Some(interval) = created {
            let datetime = Utc::now() - interval.delta();
            let target = format!("{}", datetime.format("%Y-%m-%dT%H:%M:%SZ"));
            params.push(("new_since".to_string(), target));
        }

        if !params.is_empty() {
            url = Url::parse_with_params(url.as_str(), params)?;
        }

        debug!("history request url: {url}");
        Ok(Self(service.client().get(url).build()?))
    }
}

impl Request for HistoryRequest {
    type Output = Vec<Vec<Event>>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.0).await?;
        let mut data = service.parse_response(response).await?;
        debug!("history request data: {data}");
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
