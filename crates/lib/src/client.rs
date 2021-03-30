use serde_json::Value;
use tracing::{debug, info};

use crate::service::bugzilla::Bug;
use crate::service::{Config, Service};
use crate::traits::{Params, WebService};
use crate::Error;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub struct ClientBuilder {
    client: reqwest::ClientBuilder,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder().user_agent(USER_AGENT),
        }
    }

    pub fn build(self, config: Config) -> crate::Result<Client> {
        Ok(Client {
            service: config.service(self.client.build()?),
        })
    }
}

pub struct Client {
    service: Service,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn service(&self) -> &Service {
        &self.service
    }

    pub async fn get<S>(
        &self,
        ids: &[S],
        comments: bool,
        attachments: bool,
    ) -> crate::Result<Vec<Bug>>
    where
        S: std::fmt::Display,
    {
        let mut futures = vec![];
        for id in ids {
            let request = self.service.get_request(id, comments, attachments)?;
            info!("get request: {}", request.url());
            futures.push(self.service.client().execute(request));
        }

        let mut bugs = vec![];
        for future in futures {
            let response = future.await?;
            let body = response.text().await?;
            let mut json: Value = body.parse()?;
            if json.get("error").is_some() {
                let code = json["code"].as_i64().unwrap();
                let message = json["message"].as_str().unwrap().to_string();
                return Err(Error::Bugzilla { code, message });
            } else {
                let data = json["bugs"][0].take();
                debug!("get data: {data}");
                bugs.push(serde_json::from_value(data)?);
            }
        }

        Ok(bugs)
    }

    pub async fn search<Q: Params>(&self, query: Q) -> crate::Result<Vec<Bug>> {
        let request = self.service.search_request(query)?;
        info!("search request: {}", request.url());
        let response = self.service.client().execute(request).await?;
        let body = response.text().await?;
        let mut json: Value = body.parse()?;
        if json.get("error").is_some() {
            let code = json["code"].as_i64().unwrap();
            let message = json["message"].as_str().unwrap().to_string();
            Err(Error::Bugzilla { code, message })
        } else {
            let data = json["bugs"].take();
            debug!("get data: {data}");
            Ok(serde_json::from_value(data)?)
        }
    }
}
