use indexmap::IndexMap;
use serde::Deserialize;
use url::Url;

use crate::Error;
use crate::objects::bugzilla::Comment;
use crate::service::bugzilla::Bugzilla;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, ParseResponse, RequestSend};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<String>,
    pub params: Parameters,
}

impl Request {
    pub(crate) fn new<I, S>(service: Bugzilla, ids: I) -> Self
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
            .config()
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

    pub fn attachment(&mut self, value: bool) -> &mut Self {
        self.params.attachment = Some(value);
        self
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

#[derive(Deserialize, Debug)]
struct BugComments {
    comments: Vec<Comment>,
}

/// Bugzilla REST API response to a comment get request.
///
/// https://bugzilla.readthedocs.io/en/latest/api/core/v1/comment.html#get-comments
#[derive(Deserialize, Debug)]
struct Response {
    bugs: IndexMap<u64, BugComments>,
    #[serde(rename = "comments")]
    _comments: IndexMap<u64, Comment>,
}

impl RequestSend for Request {
    type Output = Vec<Vec<Comment>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let data: Response = self.service.parse_response(response).await?;

        // Bugzilla's response always uses bug IDs even if attachments were requested via
        // alias so we assume the response is in the same order as the request.
        let mut comments = vec![];
        for mut bug in data.bugs.into_values() {
            // filter comments
            bug.comments.retain(|x| self.params.filter(x));
            comments.push(bug.comments);
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
        if let Some(value) = self.attachment
            && comment.attachment_id.is_some() != value
        {
            return false;
        }

        if let Some(value) = self.creator.as_ref()
            && !comment.creator.contains(value)
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.comment_get(ids).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no IDs specified");

        server.reset().await;
        server
            .respond(200, path.join("comment/get/multiple-bugs.json"))
            .await;

        let comments = service.comment_get([1, 2]).send().await.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].len(), 2);
        assert_eq!(comments[1].len(), 1);

        server.reset().await;
        server
            .respond(200, path.join("comment/get/single-bug.json"))
            .await;

        // all comments
        let comments = service.comment_get([1]).send().await.unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 2, 3, 4, 5, 6, 7]);

        // comments with attachments
        let comments = service
            .comment_get([1])
            .attachment(true)
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [2, 3, 4]);

        // comments without attachments
        let comments = service
            .comment_get([1])
            .attachment(false)
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 5, 6, 7]);

        // comments with time bounds
        let value = "2020".parse().unwrap();
        let comments = service
            .comment_get([1])
            .created_after(value)
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 2, 3, 4, 5, 6, 7]);

        // comments by a specific user
        let comments = service
            .comment_get([1])
            .creator("user1")
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [1, 2, 3, 7]);

        // comments with attachments by a specific user
        let comments = service
            .comment_get([1])
            .attachment(true)
            .creator("user2")
            .send()
            .await
            .unwrap();
        assert_ordered_eq!(comments[0].iter().map(|x| x.id), [4]);
    }
}
