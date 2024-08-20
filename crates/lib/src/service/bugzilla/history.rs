use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Event;
use crate::service::bugzilla::Service;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    ids: Vec<String>,
    params: Parameters,
}

impl<'a> Request<'a> {
    pub(crate) fn new<I, S>(service: &'a Service, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service,
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            params: Default::default(),
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = self
            .service
            .config
            .base
            .join(&format!("rest/bug/{id}/history"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in &self.ids[1..] {
            url.query_pairs_mut().append_pair("ids", id);
        }

        if let Some(value) = self.params.created_after.as_ref() {
            url.query_pairs_mut()
                .append_pair("new_since", value.as_ref());
        }

        Ok(url)
    }

    pub fn params(mut self, params: Parameters) -> Self {
        self.params = params;
        self
    }

    pub fn created_after(mut self, interval: TimeDeltaOrStatic) -> Self {
        self.params.created_after = Some(interval);
        self
    }

    pub fn creator<S>(mut self, value: S) -> Self
    where
        S: Into<String>,
    {
        self.params.creator = Some(value.into());
        self
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<Vec<Event>>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(self.service);
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
    fn filter(&self, event: &Event) -> bool {
        if let Some(value) = self.creator.as_ref() {
            if !event.who.contains(value) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.history(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }
}
