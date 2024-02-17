use std::fs;

use camino::Utf8Path;
use serde::Deserialize;

use crate::service::{self, ServiceKind};
use crate::Error;

/// Config support
#[derive(Debug, Deserialize)]
pub struct Config {
    default: Option<String>,
    connections: Vec<Connection>,
}

impl Config {
    /// Load connection configuration from a given file path.
    pub fn load(path: &Utf8Path) -> crate::Result<Self> {
        let data = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;
        let config = toml::from_str(&data)
            .map_err(|e| Error::Config(format!("failed parsing config: {path}: {e}")))?;
        Ok(config)
    }

    /// Get the config's default connection.
    pub fn default(&self) -> Option<&str> {
        self.default.as_deref()
    }

    /// Get all the config's connections.
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }
}

/// Connection config support
#[derive(Debug, Deserialize)]
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

    /// Create a service config from the connection.
    pub fn service(&self) -> crate::Result<service::Config> {
        self.service.create(&self.base)
    }
}
