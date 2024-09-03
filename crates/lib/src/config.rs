use std::fs;
use std::ops::Deref;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::service::{self, ServiceKind};
use crate::Error;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!("../../../data/services.toml");

/// Connection config support.
#[derive(Deserialize, Serialize, Debug)]
pub struct Config(IndexMap<String, service::Config>);

impl Config {
    pub fn new() -> crate::Result<Self> {
        let connections = toml::from_str(SERVICES_DATA)
            .map_err(|e| Error::Config(format!("failed loading bundled service data: {e}")))?;
        Ok(connections)
    }

    /// Load connections from a given file path.
    pub fn load<P: AsRef<Utf8Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;
        let config = toml::from_str(&data)
            .map_err(|e| Error::Config(format!("failed parsing config: {path}: {e}")))?;
        Ok(config)
    }

    pub fn get(&self, kind: ServiceKind, name: &str) -> crate::Result<service::Config> {
        if ["https://", "http://"].iter().any(|s| name.starts_with(s)) {
            service::Config::new(kind, name)
        } else {
            self.0
                .get(name)
                .cloned()
                .ok_or_else(|| Error::InvalidValue(format!("unknown connection: {name}")))
        }
    }
}

impl Deref for Config {
    type Target = IndexMap<String, service::Config>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Config {
    type Item = (String, service::Config);
    type IntoIter = indexmap::map::IntoIter<String, service::Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Config {
    type Item = (&'a String, &'a service::Config);
    type IntoIter = indexmap::map::Iter<'a, String, service::Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
