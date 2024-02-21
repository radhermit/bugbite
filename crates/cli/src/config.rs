use anyhow::anyhow;
use bugbite::service::{self, ServiceKind};
use bugbite::services::SERVICES;
use camino::Utf8Path;
use indexmap::IndexMap;

#[derive(Debug, Default)]
pub(crate) struct Config {
    pub(crate) default: Option<String>,
    pub(crate) connections: IndexMap<String, service::Config>,
}

impl TryFrom<bugbite::config::Config> for Config {
    type Error = anyhow::Error;

    fn try_from(config: bugbite::config::Config) -> anyhow::Result<Self> {
        let mut connections = IndexMap::new();
        for c in config.connections() {
            connections.insert(c.name().to_string(), c.service()?);
        }

        Ok(Self {
            default: config.default().map(|s| s.to_string()),
            connections,
        })
    }
}

impl Config {
    pub(crate) fn load(path: Option<&Utf8Path>) -> anyhow::Result<Self> {
        // TODO: fallback to loading from a default user path
        if let Some(path) = path {
            let config = bugbite::config::Config::load(path)?;
            config.try_into()
        } else {
            Ok(Self::default())
        }
    }

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
