use serde_json::Value;
use url::Url;

use crate::Error;
use crate::objects::bugzilla::Event;
use crate::service::bugzilla::Bugzilla;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<String>,
    pub params: Parameters,
}

impl Request {
    pub(super) fn new<I, S>(service: &Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service: service.clone(),
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
            .config()
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

    pub fn created_after(&mut self, interval: TimeDeltaOrStatic) -> &mut Self {
        self.params.created_after = Some(interval);
        self
    }

    pub fn creator<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.params.creator = Some(value.into());
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<Vec<Event>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(bugs) = data["bugs"].take() else {
            return Err(Error::InvalidResponse("history request".to_string()));
        };

        let mut history = vec![];

        for mut bug in bugs {
            let Value::Array(data) = bug["history"].take() else {
                return Err(Error::InvalidResponse("history request".to_string()));
            };

            // deserialize and filter events
            let mut bug_history = vec![];
            for value in data {
                let event: Event = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidResponse(format!("failed deserializing event: {e}"))
                })?;
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
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.history(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        server.reset().await;
        server
            .respond(200, path.join("history/multiple-bugs.json"))
            .await;

        let changes = service.history([1, 2]).send().await.unwrap();
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].len(), 3);
        assert_eq!(changes[1].len(), 1);

        server.reset().await;
        server
            .respond(200, path.join("history/single-bug.json"))
            .await;

        // all changes
        let changes = service.history([1]).send().await.unwrap();
        assert_eq!(changes[0].len(), 3);

        // changes by a specific user
        let changes = service.history([1]).creator("user1").send().await.unwrap();
        assert_eq!(changes[0].len(), 2);
    }
}
