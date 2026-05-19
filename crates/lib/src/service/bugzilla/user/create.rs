use serde::Serialize;
use serde_with::skip_serializing_none;
use url::Url;

use crate::Error;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub emails: Vec<String>,
    pub name: Option<String>,
    pub password: Option<String>,
}

impl Request {
    pub(crate) fn new<I, S>(service: &Bugzilla, emails: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: std::fmt::Display,
    {
        Self {
            service: service.clone(),
            emails: emails.into_iter().map(|s| s.to_string()).collect(),
            name: Default::default(),
            password: Default::default(),
        }
    }

    fn url(&self) -> crate::Result<Url> {
        let url = self.service.config().base.join("rest/user")?;

        Ok(url)
    }

    /// Set the user name.
    pub fn name<S: std::fmt::Display>(mut self, value: Option<S>) -> Self {
        self.name = value.map(|s| s.to_string());
        self
    }

    /// Set the user password.
    pub fn password<S: std::fmt::Display>(mut self, value: Option<S>) -> Self {
        self.password = value.map(|s| s.to_string());
        self
    }

    fn params<'a>(&'a self, email: &'a str) -> RequestUser<'a> {
        RequestUser {
            email,
            name: self.name.as_deref(),
            password: self.password.as_deref(),
        }
    }
}

/// User creation parameters used for request submission.
#[skip_serializing_none]
#[derive(Serialize, Debug)]
struct RequestUser<'a> {
    email: &'a str,
    name: Option<&'a str>,
    password: Option<&'a str>,
}

impl RequestSend for Request {
    type Output = Vec<u64>;

    async fn send(&self) -> crate::Result<Self::Output> {
        if self.emails.is_empty() {
            return Err(Error::InvalidRequest("no emails specified".to_string()));
        }
        let url = self.url()?;

        let mut futures = vec![];
        for email in &self.emails {
            let params = self.params(email);
            futures.push(
                self.service
                    .client()
                    .post(url.clone())
                    .json(&params)
                    .auth(&self.service)?
                    .send(),
            )
        }

        let mut user_ids = vec![];
        for future in futures {
            let response = future.await?;
            let mut data = self.service.parse_response(response).await?;
            let id = data["id"].take();
            let id = serde_json::from_value(id)
                .map_err(|e| Error::InvalidResponse(format!("failed deserializing id: {e}")))?;
            user_ids.push(id);
        }

        Ok(user_ids)
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

        // no emails
        let emails = Vec::<&str>::new();
        let err = service.user_create(emails).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no emails specified");

        // single user, unauthenticated session
        server
            .respond(200, path.join("user/create/single.json"))
            .await;
        let err = service
            .user_create(["user@domain.com"])
            .send()
            .await
            .unwrap_err();
        assert_err_re!(err, "authentication required");

        server.reset().await;

        // create authenticated service
        let service = Bugzilla::builder(server.uri())
            .unwrap()
            .user("test")
            .password("test")
            .build()
            .unwrap();

        // invalid user
        server
            .respond(400, path.join("user/create/invalid-user.json"))
            .await;
        let err = service.user_create(["test"]).send().await.unwrap_err();
        assert_err_re!(err, "e-mail address");

        server.reset().await;

        // single user, authenticated session
        server
            .respond(201, path.join("user/create/single.json"))
            .await;
        let id = service
            .user_create(["user@domain.com"])
            .send()
            .await
            .unwrap()[0];
        assert_eq!(id, 2);
    }
}
