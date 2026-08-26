use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use url::Url;

use crate::Error;
use crate::objects::bugzilla::Comment;
pub use crate::objects::{SetChange, SetChanges};
use crate::service::bugzilla::Bugzilla;
use crate::time::TimeDeltaOrStatic;
use crate::traits::{InjectAuth, Merge, RequestSend, RequestTemplate, WebService};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Request {
    #[serde(skip)]
    service: Bugzilla,
    #[serde(skip)]
    pub ids: Vec<String>,
    #[serde(flatten)]
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

    fn url(&self, comment: &Comment) -> crate::Result<Url> {
        let id = comment.id;
        let url = self
            .service
            .config()
            .base
            .join(&format!("rest/bug/comment/{id}/tags"))?;
        Ok(url)
    }

    /// Encode parameters into the form required for the request.
    fn encode<'a>(&'a self, comment: &'a Comment) -> crate::Result<RequestParameters<'a>> {
        // verify parameters exist
        if self.params == Parameters::default() {
            return Err(Error::EmptyParams);
        }

        let mut params = RequestParameters {
            comment_id: comment.id,
            ..Default::default()
        };

        // tags are guaranteed to exist by earlier check
        let tags = self.params.tags.as_ref().expect("no tags exist");

        let changes: SetChanges<_> = tags.iter().collect();
        if let Some(tags) = &changes.set {
            params.add = Some(tags.to_vec());
            params.remove = Some(comment.tags.iter().collect());
        } else {
            params.add = changes.add;
            params.remove = changes.remove;
        }

        Ok(params)
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

    pub fn tags<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = SetChange<String>>,
    {
        self.params.tags = Some(values.into_iter().collect());
        self
    }

    pub fn untag(&mut self, value: bool) -> &mut Self {
        if value {
            self.params.tags = Some(Default::default());
        }
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<Vec<String>>;

    async fn send(&self) -> crate::Result<Self::Output> {
        if self.ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        }

        // tags have to exist, to unset tags it should be empty
        if self.params.tags.is_none() {
            return Err(Error::InvalidRequest("no tags specified".to_string()));
        }

        // get the matching comments
        let comments = self.service.comment_get(&self.ids).send().await?;
        let comments = comments.into_iter().flatten();

        // send comment tag update requests
        let mut requests = vec![];
        for comment in comments {
            let url = self.url(&comment)?;
            let params = self.encode(&comment)?;
            let request = self
                .service
                .client()
                .put(url)
                .json(&params)
                .auth(&self.service)?;
            requests.push(request.send());
        }

        let mut tags = vec![];
        for request in requests {
            let response = request.await?;
            let data = self.service.parse_response(response).await?;
            let comment_tags: Vec<String> = serde_json::from_value(data)
                .map_err(|e| Error::InvalidResponse(format!("tag request: {e}")))?;
            tags.push(comment_tags);
        }

        Ok(tags)
    }
}

impl RequestTemplate for Request {
    type Params = Parameters;
    type Service = Bugzilla;
    const TYPE: &'static str = "update";

    fn service(&self) -> &Self::Service {
        &self.service
    }

    fn params(&mut self) -> &mut Self::Params {
        &mut self.params
    }
}

/// Construct bug comment parameters.
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub attachment: Option<bool>,
    pub created_after: Option<TimeDeltaOrStatic>,
    pub creator: Option<String>,
    pub tags: Option<Vec<SetChange<String>>>,
}

impl Merge for Parameters {
    fn merge(&mut self, other: Self) {
        *self = Self {
            attachment: self.attachment.merge(other.attachment),
            created_after: self.created_after.merge(other.created_after),
            creator: self.creator.merge(other.creator),
            tags: self.tags.merge(other.tags),
        }
    }
}

/// Internal comment tag request parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/comment.html#update-comment-tags for
/// more information.
#[skip_serializing_none]
#[derive(Serialize, Default)]
struct RequestParameters<'a> {
    comment_id: u64,
    add: Option<Vec<&'a String>>,
    remove: Option<Vec<&'a String>>,
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use indexmap::IndexSet;
    use wiremock::matchers;

    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let path = TESTDATA_PATH.join("bugzilla");
        let server = TestServer::new().await;
        let service = Bugzilla::builder(server.uri())
            .unwrap()
            .user("user")
            .password("pass")
            .build()
            .unwrap();

        // no IDs
        let ids = Vec::<u32>::new();
        let err = service.comment_tag(ids).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no IDs specified");

        // no tags
        let err = service.comment_tag([1]).send().await.unwrap_err();
        assert_matches!(err, Error::InvalidRequest(_));
        assert_err_re!(err, "no tags specified");

        server.reset().await;
        server
            .respond_match(
                matchers::path("/rest/bug/1/comment"),
                200,
                path.join("comment/get/single-bug.json"),
            )
            .await;
        server
            .respond_match(
                matchers::path_regex("/rest/bug/comment/.*"),
                200,
                path.join("comment/tag/nonexistent.json"),
            )
            .await;

        // nonexistent tags
        let tag = "+tag".parse().unwrap();
        let tags = service.comment_tag([1]).tags([tag]).send().await.unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);

        server.reset().await;
        server
            .respond_match(
                matchers::path("/rest/bug/1/comment"),
                200,
                path.join("comment/get/single-bug.json"),
            )
            .await;
        server
            .respond_match(
                matchers::path_regex("/rest/bug/comment/.*"),
                200,
                path.join("comment/tag/single.json"),
            )
            .await;

        // set single tag
        let tag = "tag".parse().unwrap();
        let tags = service.comment_tag([1]).tags([tag]).send().await.unwrap();
        // all seven comments have the same tag
        assert_eq!(tags.len(), 7);
        let tags: IndexSet<_> = tags.iter().flatten().collect();
        assert_ordered_eq!(tags, ["tag"]);

        server.reset().await;
        server
            .respond_match(
                matchers::path("/rest/bug/1/comment"),
                200,
                path.join("comment/get/single-bug.json"),
            )
            .await;
        server
            .respond_match(
                matchers::path_regex("/rest/bug/comment/.*"),
                200,
                path.join("comment/tag/nonexistent.json"),
            )
            .await;

        // comments with attachments
        let tags = service
            .comment_tag([1])
            .attachment(true)
            .untag(true)
            .send()
            .await
            .unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);

        // comments without attachments
        let tags = service
            .comment_tag([1])
            .attachment(false)
            .untag(true)
            .send()
            .await
            .unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);

        // comments with time bounds
        let value = "2020".parse().unwrap();
        let tags = service
            .comment_tag([1])
            .created_after(value)
            .untag(true)
            .send()
            .await
            .unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);

        // comments by a specific user
        let tags = service
            .comment_tag([1])
            .creator("user1")
            .untag(true)
            .send()
            .await
            .unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);

        // comments with attachments by a specific user
        let tags = service
            .comment_tag([1])
            .attachment(true)
            .creator("user2")
            .untag(true)
            .send()
            .await
            .unwrap();
        assert_eq!(tags.iter().flatten().count(), 0);
    }
}
