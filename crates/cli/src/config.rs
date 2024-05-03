use anyhow::anyhow;
use bugbite::service::{self, ServiceKind};
use bugbite::services::SERVICES;
use camino::Utf8Path;
use indexmap::IndexMap;

#[derive(Debug, Default)]
pub(crate) struct Config {
    pub(crate) connections: IndexMap<String, service::Config>,
}

impl TryFrom<bugbite::config::Config> for Config {
    type Error = anyhow::Error;

    fn try_from(config: bugbite::config::Config) -> anyhow::Result<Self> {
        let mut connections = IndexMap::new();
        for c in config.connections() {
            connections.insert(
                c.name().to_string(),
                service::Config::new(c.kind(), c.base())?,
            );
        }

        Ok(Self { connections })
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

    /// Return a pre-configured service URL by its connection name and kind.
    pub(crate) fn get_kind<'a>(&'a self, kind: ServiceKind, name: &str) -> anyhow::Result<&'a str> {
        match (self.connections.get(name), SERVICES.get(name)) {
            (Some(service), _) | (_, Some(service)) => {
                if service.kind() == kind {
                    Ok(service.base().as_str())
                } else {
                    Err(anyhow!("incompatible connection: {name}"))
                }
            }
            _ => Err(anyhow!("unknown connection: {name}")),
        }
    }
}
