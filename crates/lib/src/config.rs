use std::fs;

use camino::Utf8Path;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::service;
use crate::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    services: IndexMap<String, service::Config>,
}

impl Config {
    pub fn try_new<P: AsRef<Utf8Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;

        let services = toml::from_str(&data)
            .map_err(|e| Error::Config(format!("failed loading config: {path}: {e}")))?;

        Ok(Self { services })
    }

    pub fn get(&self, name: &str) -> crate::Result<&service::Config> {
        self.services
            .get(name)
            .ok_or_else(|| Error::Config(format!("unknown service: {name}")))
    }
}

/// Pre-defined services.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    use crate::service::ServiceKind::*;
    Config {
        services: [
            (
                "gentoo",
                BugzillaRestV1.create("https://bugs.gentoo.org/").unwrap(),
            ),
            (
                "gcc",
                BugzillaRestV1
                    .create("https://gcc.gnu.org/bugzilla/")
                    .unwrap(),
            ),
            (
                "linux",
                BugzillaRestV1
                    .create("https://bugzilla.kernel.org/")
                    .unwrap(),
            ),
            (
                "pkgcraft",
                Github
                    .create("https://github.com/pkgcraft/pkgcraft/")
                    .unwrap(),
            ),
        ]
        .into_iter()
        .map(|(name, service)| (name.to_string(), service))
        .collect(),
    }
});
