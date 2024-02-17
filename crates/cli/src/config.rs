use anyhow::anyhow;
use bugbite::service::{self, ServiceKind};
use bugbite::services::{DEFAULT, SERVICES};
use indexmap::IndexMap;

pub(crate) struct Config {
    pub(crate) default: Option<String>,
    pub(crate) connections: IndexMap<String, service::Config>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default: Some(DEFAULT.to_string()),
            connections: Default::default(),
        }
    }
}

impl Config {
    pub(crate) fn get(&self, name: &str) -> anyhow::Result<(ServiceKind, String)> {
        match (self.connections.get(name), SERVICES.get(name)) {
            (Some(service), _) | (_, Some(service)) => {
                Ok((service.kind(), service.base().to_string()))
            }
            _ => Err(anyhow!("unknown service: {name}")),
        }
    }

    pub(crate) fn get_default(&self) -> anyhow::Result<(ServiceKind, String)> {
        let name = self
            .default
            .as_ref()
            .ok_or_else(|| anyhow!("no default connection configured"))?;
        self.get(name)
    }
}
