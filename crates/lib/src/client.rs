use enum_as_inner::EnumAsInner;

use crate::service::Config;

pub mod bugzilla;
pub mod github;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
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
        let client = match config {
            Config::BugzillaRestV1(config) => {
                Client::Bugzilla(bugzilla::Client::new(config, self.client)?)
            }
            Config::Github(config) => Client::Github(github::Client::new(config, self.client)?),
        };

        Ok(client)
    }
}

#[derive(EnumAsInner, Debug)]
pub enum Client {
    Bugzilla(bugzilla::Client),
    Github(github::Client),
}
