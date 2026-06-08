use serde_json::Value;
use url::Url;

use crate::Error;
use crate::objects::bugzilla::User;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<String>,
    pub groups: Vec<String>,
    pub disabled: Option<bool>,
}

impl Request {
    pub(crate) fn new<I, S>(service: &Bugzilla, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service: service.clone(),
            ids: ids.into_iter().map(|s| s.to_string()).collect(),
            groups: Default::default(),
            disabled: Default::default(),
        }
    }

    fn url(&self) -> crate::Result<Url> {
        if self.ids.is_empty() {
            return Err(Error::InvalidRequest("no IDs specified".to_string()));
        }

        let mut url = self.service.config().base.join("rest/user")?;

        for id in &self.ids {
            // determine user variant
            let user_kind = if id.parse::<u64>().is_ok() {
                "ids"
            } else if id.contains("@") {
                "names"
            } else {
                "match"
            };
            url.query_pairs_mut().append_pair(user_kind, id);
        }

        for group in &self.groups {
            // determine group variant
            let group_kind = if group.parse::<u64>().is_ok() {
                "group_ids"
            } else {
                "groups"
            };
            url.query_pairs_mut().append_pair(group_kind, group);
        }

        if let Some(value) = self.disabled {
            url.query_pairs_mut()
                .append_pair("include_disabled", &value.to_string());
        }

        Ok(url)
    }

    pub fn groups<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        self.groups = values.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = Some(value);
        self
    }
}

impl RequestSend for Request {
    type Output = Vec<User>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let request = self
            .service
            .client()
            .get(self.url()?)
            .auth_optional(&self.service);
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(data) = data["users"].take() else {
            return Err(Error::InvalidResponse("user get request".to_string()));
        };

        let mut users = vec![];
        for value in data {
            let user: User = serde_json::from_value(value)
                .map_err(|e| Error::InvalidResponse(format!("failed deserializing user: {e}")))?;
            users.push(user);
        }

        Ok(users)
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
        let ids = Vec::<u64>::new();
        let err = service.user_get(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");

        // nonexistent email
        server
            .respond(200, path.join("user/get/nonexistent-email.json"))
            .await;
        let err = service
            .user_get(["nonexistent@domain.com"])
            .send()
            .await
            .unwrap_err();
        assert_err_re!(err, "There is no user named 'nonexistent@domain.com'.");

        server.reset().await;

        // nonexistent id
        server
            .respond(200, path.join("user/get/nonexistent-id.json"))
            .await;
        let users = service.user_get([123]).send().await.unwrap();
        assert_eq!(users, []);

        server.reset().await;

        // single user, unauthenticated session
        server
            .respond(200, path.join("user/get/single-unauthenticated.json"))
            .await;
        let user = &service.user_get(["user@domain.com"]).send().await.unwrap()[0];
        assert_eq!(user.id, 123);
        assert_eq!(user.real_name.as_deref(), Some("A User"));
        assert_eq!(user.name, "user");
    }
}
