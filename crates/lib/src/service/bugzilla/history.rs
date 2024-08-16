use serde_json::Value;

use crate::objects::bugzilla::Event;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a super::Service,
    url: url::Url,
    params: Parameters,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I, S>(service: &'a super::Service, ids: I) -> crate::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        let mut ids = ids.into_iter().map(|s| s.to_string());
        let id = ids
            .next()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = service
            .config
            .base
            .join(&format!("rest/bug/{id}/history"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in ids {
            url.query_pairs_mut().append_pair("ids", &id);
        }

        Ok(Self {
            service,
            url,
            params: Default::default(),
        })
    }

    pub fn params(mut self, params: Parameters) -> Self {
        if let Some(value) = params.created_after.as_ref() {
            self.url
                .query_pairs_mut()
                .append_pair("new_since", value.as_ref());
        }

        self.params = params;
        self
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<Vec<Event>>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url)
            .auth_optional(self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(bugs) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to history request".to_string(),
            ));
        };

        let mut history = vec![];

        for mut bug in bugs {
            let Value::Array(data) = bug["history"].take() else {
                return Err(Error::InvalidValue(
                    "invalid service response to history request".to_string(),
                ));
            };

            // deserialize and filter events
            let mut bug_history = vec![];
            for value in data {
                let event: Event = serde_json::from_value(value)
                    .map_err(|e| Error::InvalidValue(format!("failed deserializing event: {e}")))?;
                if self.params.filter(&event) {
                    bug_history.push(event);
                }
            }

            history.push(bug_history);
        }

        Ok(history)
    }
}

/// Construct bug history parameters.
#[derive(Debug, Default)]
pub struct Parameters {
    pub created_after: Option<TimeDeltaOrStatic>,
    pub creator: Option<String>,
}

impl Parameters {
    pub fn new() -> Self {
        Self::default()
    }

    fn filter(&self, event: &Event) -> bool {
        if let Some(value) = self.creator.as_ref() {
            if !event.who.contains(value) {
                return false;
            }
        }

        true
    }

    pub fn created_after(&mut self, interval: TimeDeltaOrStatic) {
        self.created_after = Some(interval);
    }

    pub fn creator<S>(&mut self, value: S)
    where
        S: Into<String>,
    {
        self.creator = Some(value.into());
    }
}
