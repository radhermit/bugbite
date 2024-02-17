use std::time::Duration;

use enum_as_inner::EnumAsInner;

use crate::service::Config;

pub mod bugzilla;
pub mod github;

static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Default)]
pub struct ClientBuilder {
    timeout: u64,
    insecure: bool,
}

impl ClientBuilder {
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }

    pub fn build(self, config: Config) -> crate::Result<Client> {
        let builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(self.timeout))
            .danger_accept_invalid_certs(self.insecure);

        let client = match config {
            Config::BugzillaRestV1(config) => {
                Client::Bugzilla(bugzilla::Client::new(config, builder)?)
            }
            Config::Github(config) => Client::Github(github::Client::new(config, builder)?),
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
        ClientBuilder::default()
    }
}
