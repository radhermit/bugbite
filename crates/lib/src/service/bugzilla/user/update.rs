use itertools::{Either, Itertools};
use serde::Serialize;
use serde_json::Value;
use serde_with::skip_serializing_none;
use url::Url;

use crate::Error;
use crate::service::bugzilla::Bugzilla;
use crate::traits::{InjectAuth, RequestSend, WebService};

#[derive(Debug)]
pub struct Request {
    service: Bugzilla,
    pub ids: Vec<String>,
    pub params: Parameters,
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
            params: Default::default(),
        }
    }

    /// Encode parameters into the form required for the request.
    fn encode(&self) -> crate::Result<RequestParameters<'_>> {
        // verify parameters exist
        if self.params == Parameters::default() {
            return Err(Error::EmptyParams);
        }

        let (ids, names): (Vec<u64>, Vec<&str>) =
            self.ids.iter().partition_map(|s| match s.parse::<u64>() {
                Ok(n) => Either::Left(n),
                Err(_) => Either::Right(s.as_str()),
            });

        Ok(RequestParameters {
            ids,
            names,
            full_name: self.params.name.as_deref(),
            email: self.params.email.as_deref(),
            password: self.params.password.as_deref(),
            email_enabled: self.params.email_enabled,
            login_denied_text: self.params.disable.as_deref(),
        })
    }

    fn url(&self) -> crate::Result<Url> {
        let id = self
            .ids
            .first()
            .ok_or_else(|| Error::InvalidRequest("no IDs specified".to_string()))?;

        let url = self
            .service
            .config()
            .base
            .join(&format!("rest/user/{id}"))?;

        Ok(url)
    }
}

impl RequestSend for Request {
    type Output = Vec<u64>;

    async fn send(&self) -> crate::Result<Self::Output> {
        let url = self.url()?;
        let params = self.encode()?;
        let request = self
            .service
            .client()
            .put(url)
            .json(&params)
            .auth(&self.service)?;
        let response = request.send().await?;
        let mut data = self.service.parse_response(response).await?;
        let Value::Array(data) = data["users"].take() else {
            return Err(Error::InvalidResponse("user update request".to_string()));
        };

        let mut ids = vec![];
        for mut change in data {
            let id = serde_json::from_value(change["id"].take()).map_err(|e| {
                Error::InvalidResponse(format!("failed deserializing changes: {e}"))
            })?;
            ids.push(id);
        }

        Ok(ids)
    }
}

/// User update parameters.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parameters {
    /// User's real name.
    pub name: Option<String>,

    /// User's email address.
    pub email: Option<String>,

    /// User's password.
    pub password: Option<String>,

    /// Enable or disable sending bug-related email.
    pub email_enabled: Option<bool>,

    /// Disable user account with reason.
    pub disable: Option<String>,
}

/// Internal user update request parameters.
#[skip_serializing_none]
#[derive(Serialize)]
struct RequestParameters<'a> {
    ids: Vec<u64>,
    names: Vec<&'a str>,
    full_name: Option<&'a str>,
    email: Option<&'a str>,
    password: Option<&'a str>,
    email_enabled: Option<bool>,
    login_denied_text: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    use super::*;

    #[tokio::test]
    async fn request() {
        let server = TestServer::new().await;
        let service = Bugzilla::new(server.uri()).unwrap();

        // no IDs
        let ids = Vec::<u64>::new();
        let err = service.user_update(ids).send().await.unwrap_err();
        assert!(matches!(err, Error::InvalidRequest(_)));
        assert_err_re!(err, "no IDs specified");
    }
}
