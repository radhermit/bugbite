use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Bug;
use crate::service::bugzilla::Service;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

use super::{attachment, comment, history};

#[derive(Debug)]
pub struct Request<'a> {
    service: &'a Service,
    ids: Vec<String>,
    attachments: Option<attachment::get_item::Request<'a>>,
    comments: Option<comment::Request<'a>>,
    history: Option<history::Request<'a>>,
}

impl<'a> Request<'a> {
    pub(super) fn new<I, S>(service: &'a Service, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service,
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            attachments: None,
            comments: None,
            history: None,
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let mut url = self.service.config.base.join(&format!("rest/bug/{id}"))?;

        // Note that multiple request support is missing from upstream's REST API
        // documentation, but exists in older RPC-based docs.
        for id in &self.ids[1..] {
            url.query_pairs_mut().append_pair("ids", id);
        }

        // include personal tags
        url.query_pairs_mut()
            .append_pair("include_fields", "_default,tags");

        // drop useless token that is injected for authenticated requests
        url.query_pairs_mut()
            .append_pair("exclude_fields", "update_token");

        Ok(url)
    }

    /// Enable or disable fetching attachments.
    pub fn attachments(mut self, fetch: bool) -> Self {
        if fetch {
            self.attachments = Some(self.service.attachment_get_item(&self.ids).data(false));
        }
        self
    }

    /// Enable or disable fetching comments.
    pub fn comments(mut self, fetch: bool) -> Self {
        if fetch {
            self.comments = Some(self.service.comment(&self.ids));
        }
        self
    }

    /// Enable or disable fetching changes.
    pub fn history(mut self, fetch: bool) -> Self {
        if fetch {
            self.history = Some(self.service.history(&self.ids));
        }
        self
    }
}

impl RequestSend for Request<'_> {
    type Output = Vec<Bug>;

    async fn send(self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(self.service);

        // send data requests
        let attachments = self.attachments.map(|r| r.send());
        let comments = self.comments.map(|r| r.send());
        let history = self.history.map(|r| r.send());

        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(data) = data["bugs"].take() else {
            return Err(Error::InvalidValue(
                "invalid service response to get request".to_string(),
            ));
        };

        // parse data requests
        let mut attachments = match attachments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut comments = match comments {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };
        let mut history = match history {
            Some(f) => f.await?.into_iter(),
            None => Vec::new().into_iter(),
        };

        let mut bugs = vec![];
        for value in data {
            let mut bug: Bug = serde_json::from_value(value)
                .map_err(|e| Error::InvalidValue(format!("failed deserializing bug: {e}")))?;
            bug.attachments = attachments.next().unwrap_or_default();
            bug.comments = comments.next().unwrap_or_default();
            bug.history = history.next().unwrap_or_default();
            bugs.push(bug);
        }

        Ok(bugs)
    }
}

#[cfg(test)]
mod tests {
    use crate::service::bugzilla::Config;
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let config = Config::new(server.uri()).unwrap();
        let service = Service::new(config, Default::default()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.get(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent bug
        server
            .respond(404, path.join("errors/nonexistent-bug.json"))
            .await;
        let err = service.get([1]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::Bugzilla { code: 101, .. }),
            "unmatched error: {err:?}"
        );

        server.reset().await;

        // invalid response
        server.respond(200, path.join("get/invalid.json")).await;
        let err = service.get([12345]).send().await.unwrap_err();
        assert!(
            matches!(err, Error::InvalidValue(_)),
            "unmatched error: {err:?}"
        );
        assert_err_re!(err, "invalid service response");

        server.reset().await;

        // single bug
        server.respond(200, path.join("get/single-bug.json")).await;
        let ids = [12345];
        let bugs = service.get(ids).send().await.unwrap();
        assert_ordered_eq!(bugs.iter().map(|x| x.id), ids);

        server.reset().await;

        // multiple bugs
        server
            .respond(200, path.join("get/multiple-bugs.json"))
            .await;
        let ids = [12345, 23456, 34567];
        let bugs = service.get(ids).send().await.unwrap();
        assert_ordered_eq!(bugs.iter().map(|x| x.id), ids);
    }
}
