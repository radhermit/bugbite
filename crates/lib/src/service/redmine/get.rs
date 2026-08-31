use indexmap::IndexSet;
use itertools::Itertools;
use reqwest::StatusCode;
use serde::Deserialize;
use strum::Display;
use url::Url;

use crate::Error;
use crate::objects::redmine::{Issue, IssueRaw};
use crate::service::redmine::Redmine;
use crate::traits::{InjectAuth, ParseResponse, RequestSend};

#[derive(Debug)]
pub struct Request {
    service: Redmine,
    pub ids: Vec<u64>,
    fields: IndexSet<Field>,
}

impl Request {
    pub(super) fn new<I>(service: Redmine, ids: I) -> Self
    where
        I: IntoIterator<Item = u64>,
    {
        Self {
            service,
            ids: ids.into_iter().collect(),
            fields: Default::default(),
        }
    }

    fn urls(&self) -> crate::Result<Vec<Url>> {
        if self.ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        }

        let mut urls = vec![];
        for id in &self.ids {
            let mut url = self
                .service
                .config()
                .web_base()
                .join(&format!("issues/{id}.json"))?;
            if !self.fields.is_empty() {
                url.query_pairs_mut()
                    .append_pair("include", &self.fields.iter().join(","));
            }
            urls.push(url);
        }

        Ok(urls)
    }

    /// Enable or disable fetching attachments.
    pub fn attachments(&mut self, fetch: bool) -> &mut Self {
        if fetch {
            self.fields.insert(Field::Attachments);
        }
        self
    }

    /// Enable or disable fetching comments.
    pub fn comments(&mut self, fetch: bool) -> &mut Self {
        if fetch {
            self.fields.insert(Field::Journals);
        }
        self
    }
}

#[derive(Deserialize, Debug)]
struct Response {
    issue: IssueRaw,
}

/// Bug fields composed of value arrays.
#[derive(Display, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[strum(serialize_all = "snake_case")]
enum Field {
    Attachments,
    Journals,
}

impl RequestSend for Request {
    type Output = Vec<Issue>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let futures: Vec<_> = self
            .urls()?
            .into_iter()
            .map(|u| {
                self.service
                    .client()
                    .get(u)
                    .auth_optional(&self.service)
                    .send()
            })
            .collect();

        let mut issues = vec![];
        for (future, id) in futures.into_iter().zip(&self.ids) {
            let response = future.await?;
            let data: Response = match self.service.parse_response(response).await {
                Ok(data) => data,
                Err(Error::Http(StatusCode::NOT_FOUND)) => {
                    return Err(Error::InvalidValue(format!("nonexistent issue: {id}")));
                }
                Err(e) => return Err(e),
            };

            issues.push(data.issue.into());
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use wiremock::{ResponseTemplate, matchers};

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("redmine");
        let server = TestServer::new().await;
        let service = Redmine::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.get(ids).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no IDs specified");

        // nonexistent
        let template = ResponseTemplate::new(404);
        server.respond_custom(matchers::any(), template).await;
        let err = service.get([1]).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidValue(_));
        assert_err_re!(err, "nonexistent issue: 1");

        server.reset().await;

        // single
        server.respond(200, path.join("get/single.json")).await;
        let ids = [1];
        let bugs = service.get(ids).send().await.unwrap();
        assert_ordered_eq!(bugs.iter().map(|x| x.id), ids);
    }
}
