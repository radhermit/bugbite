use std::fs;

use camino::Utf8Path;
use serde::{Deserialize, Serialize};

use crate::service::ServiceKind;
use crate::Error;

/// Config support
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    connections: Vec<Connection>,
}

impl Config {
    /// Load connection configuration from a given file path.
    pub fn load<P: AsRef<Utf8Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;
        let config = toml::from_str(&data)
            .map_err(|e| Error::Config(format!("failed parsing config: {path}: {e}")))?;
        Ok(config)
    }

    /// Get all the config's connections.
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }
}

/// Connection config support
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Connection {
    name: String,
    base: String,
    service: ServiceKind,
}

impl Connection {
    /// Get the connection's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the connection's base URL.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Get the connection's service type.
    pub fn kind(&self) -> ServiceKind {
        self.service
    }
}
