use enum_as_inner::EnumAsInner;

use crate::service::Config;

pub mod bugzilla;
pub mod github;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct ClientBuilder {
    client: reqwest::ClientBuilder,
}

impl ClientBuilder {
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

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            client: reqwest::Client::builder().user_agent(USER_AGENT),
        }
    }
}
