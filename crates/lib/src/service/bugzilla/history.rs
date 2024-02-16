use itertools::Itertools;
use serde_json::Value;
use tracing::debug;
use url::Url;

use crate::objects::bugzilla::Event;
use crate::traits::{Request, WebService};
use crate::Error;

#[derive(Debug)]
pub(crate) struct HistoryRequest {
    ids: Vec<String>,
    req: reqwest::Request,
}

impl HistoryRequest {
    pub(super) fn new<S>(service: &super::Service, ids: &[S]) -> crate::Result<Self>
    where
        S: std::fmt::Display,
    {
        let mut params = vec![];
        let url = match ids {
            [id, ids @ ..] => {
                if !ids.is_empty() {
                    params.push(("ids".to_string(), ids.iter().join(",")));
                }
                format!("{}/rest/bug/{id}/history", service.base())
            }
            _ => return Err(Error::InvalidValue("invalid history IDs state".to_string())),
        };

        let url = Url::parse_with_params(&url, params)
            .map_err(|e| Error::InvalidValue(format!("invalid URL: {e}")))?;

        debug!("history request url: {url}");
        Ok(Self {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            req: service.client().get(url).build()?,
        })
    }
}

impl Request for HistoryRequest {
    type Output = Vec<Event>;
    type Service = super::Service;

    async fn send(self, service: &Self::Service) -> crate::Result<Self::Output> {
        let response = service.client().execute(self.req).await?;
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
            let events: Vec<Event> = serde_json::from_value(data)?;
            history.extend(events);
        }

        Ok(history)
    }
}
