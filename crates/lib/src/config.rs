use std::fs;
use std::ops::Deref;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::service::{self, ServiceKind};
use crate::Error;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!(concat!(env!("OUT_DIR"), "/services.toml"));

/// Connection config support.
#[derive(Deserialize, Serialize, Debug)]
pub struct Config(IndexMap<String, service::Config>);

impl Config {
    pub fn new() -> crate::Result<Self> {
        let connections = toml::from_str(SERVICES_DATA)
            .map_err(|e| Error::Config(format!("failed loading bundled service data: {e}")))?;
        Ok(connections)
    }

    /// Load connections from a given path, overriding any bundled matches.
    pub fn load<P: AsRef<Utf8Path>>(&mut self, path: P) -> crate::Result<()> {
        let load_file = |path: &Utf8Path| -> crate::Result<Self> {
            let data = fs::read_to_string(path)
                .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;
            toml::from_str(&data)
                .map_err(|e| Error::Config(format!("failed parsing config: {path}: {e}")))
        };

        let path = path.as_ref();
        let mut configs = vec![];

        if path.is_dir() {
            for entry in path.read_dir_utf8()? {
                let entry = entry?;
                configs.push(load_file(entry.path())?);
            }
        } else {
            configs.push(load_file(path)?);
        }

        // replace matching connections
        self.0.extend(configs.into_iter().flatten());
        self.0.sort_keys();

        Ok(())
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
