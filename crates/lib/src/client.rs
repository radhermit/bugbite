use std::time::Duration;

use enum_as_inner::EnumAsInner;

use crate::service::ServiceKind;

pub mod bugzilla;
pub mod github;
pub mod redmine;

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

    pub fn build(self) -> reqwest::ClientBuilder {
        reqwest::Client::builder()
            .use_rustls_tls()
            .user_agent(USER_AGENT)
            // TODO: switch to cookie_provider() once cookie (de)serialization is supported
            .cookie_store(true)
            .timeout(Duration::from_secs(self.timeout))
            .danger_accept_invalid_certs(self.insecure)
    }
}

#[derive(EnumAsInner, Debug)]
pub enum Client {
    Bugzilla(bugzilla::Client),
    Github(github::Client),
    Redmine(redmine::Client),
}

impl Client {
    pub fn new(kind: ServiceKind, base: &str) -> crate::Result<Self> {
        let builder = Client::builder().build();
        match kind {
            ServiceKind::Bugzilla => {
                let config = crate::service::bugzilla::Config::new(base)?;
                Ok(Self::Bugzilla(bugzilla::Client::new(config, builder)?))
            }
            ServiceKind::Github => {
                let config = crate::service::github::Config::new(base)?;
                Ok(Self::Github(github::Client::new(config, builder)?))
            }
            ServiceKind::Redmine => {
                let config = crate::service::github::Config::new(base)?;
                Ok(Self::Github(github::Client::new(config, builder)?))
            }
        }
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default().timeout(30)
    }
}
