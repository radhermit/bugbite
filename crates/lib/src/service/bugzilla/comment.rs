use serde_json::Value;
use url::Url;

use crate::objects::bugzilla::Comment;
use crate::service::bugzilla::Bugzilla;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, RequestSend, WebService};
use crate::Error;

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
            .config
            .base
            .join(&format!("rest/bug/{id}/comment"))?;

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

    pub fn attachment(mut self, value: bool) -> Self {
        self.params.attachment = Some(value);
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

impl RequestSend for Request {
    type Output = Vec<Vec<Comment>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let data = data["bugs"].take();
        let serde_json::value::Value::Object(data) = data else {
            return Err(Error::InvalidResponse("comment request".to_string()));
        };

        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut comments = vec![];

        for (_id, mut data) in data {
            let Value::Array(data) = data["comments"].take() else {
                return Err(Error::InvalidResponse("comment request".to_string()));
            };

            // deserialize and filter comments
            let mut bug_comments = vec![];
            for value in data {
                let comment: Comment = serde_json::from_value(value).map_err(|e| {
                    Error::InvalidResponse(format!("failed deserializing comment: {e}"))
                })?;
                if self.params.filter(&comment) {
                    bug_comments.push(comment);
                }
            }

            comments.push(bug_comments);
        }
        Ok(comments)
    }
}

/// Construct bug comment parameters.
#[derive(Debug, Default)]
pub struct Parameters {
    pub attachment: Option<bool>,
    pub created_after: Option<TimeDeltaOrStatic>,
    pub creator: Option<String>,
}

impl Parameters {
    fn filter(&self, comment: &Comment) -> bool {
        if let Some(value) = self.attachment {
            if comment.attachment_id.is_some() != value {
                return false;
            }
        }

        if let Some(value) = self.creator.as_ref() {
            if !comment.creator.contains(value) {
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
        let err = service.comment(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        server.reset().await;
        server
            .respond(200, path.join("comment/multiple-bugs.json"))
            .await;

        let comments = service.comment([1, 2]).send().await.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].len(), 2);
        assert_eq!(comments[1].len(), 1);

        server.reset().await;
        server
            .respond(200, path.join("comment/single-bug.json"))
            .await;

        // all comments
        let comments = service.comment([1]).send().await.unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 2, 3, 4, 5, 6, 7]);

        // comments with attachments
        let comments = service.comment([1]).attachment(true).send().await.unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [2, 3, 4]);

        // comments without attachments
        let comments = service.comment([1]).attachment(false).send().await.unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 5, 6, 7]);

        // comments by a specific user
        let comments = service.comment([1]).creator("user1").send().await.unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 2, 3, 7]);

        // comments with attachments by a specific user
        let comments = service
            .comment([1])
            .attachment(true)
            .creator("user2")
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [4]);
    }
}
