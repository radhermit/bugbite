use std::fs;
use std::ops::Deref;

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::service::{self, ServiceKind};
use crate::traits::try_from_toml;
use crate::Error;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!(concat!(env!("OUT_DIR"), "/services.toml"));

/// Connection config support.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config(IndexMap<String, service::Config>);

try_from_toml!(Config, "config");

impl Config {
    /// Create a new Config including bundled services.
    pub fn new() -> Self {
        toml::from_str(SERVICES_DATA)
            .unwrap_or_else(|e| panic!("failed loading bundled service data: {e}"))
    }

    /// Load connections from a given path, overriding any bundled matches.
    pub fn load<P: AsRef<Utf8Path>>(&mut self, path: P) -> crate::Result<()> {
        let path = path.as_ref();

        if path.is_dir() {
            for entry in path.read_dir_utf8()? {
                let entry = entry?;
                self.0.extend(Self::try_from(entry.path())?);
            }
        } else {
            self.0.extend(Self::try_from(path)?);
        }

        // re-sort by connection name
        self.0.sort_keys();

        Ok(())
    }

    pub fn get_kind(&self, kind: ServiceKind, name: &str) -> crate::Result<service::Config> {
        if let Some(config) = self.0.get(name).cloned() {
            Ok(config)
        } else if Url::parse(name).is_ok() {
            service::Config::new(kind, name)
        } else {
            Err(Error::InvalidValue(format!("unknown connection: {name}")))
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

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn load() {
        // bundled services only
        let mut config = Config::new();
        assert!(!config.is_empty());
        let len = config.len();

        // verify bundled gentoo connection doesn't set a user
        let c = config.get("gentoo").unwrap().as_bugzilla().unwrap();
        assert!(c.auth.user.is_none());

        let dir = tempdir().unwrap();
        let dir_path = dir.path().to_str().unwrap();
        env::set_current_dir(dir_path).unwrap();

        // create service files
        let service1 = indoc::indoc! {r#"
            [new1]
            type = "bugzilla"
            base = "https://random.bugzilla.site/"
        "#};
        fs::write("1.toml", service1).unwrap();
        let service2 = indoc::indoc! {r#"
            [new2]
            type = "redmine"
            base = "https://random.redmine.site/"
        "#};
        fs::write("2.toml", service2).unwrap();
        let gentoo = indoc::indoc! {r#"
            [gentoo]
            type = "bugzilla"
            base = "https://bugs.gentoo.org/"
            user = "user@email.com"
        "#};
        fs::write("gentoo.toml", gentoo).unwrap();

        // add new service from file
        config.load("1.toml").unwrap();
        assert!(config.len() == len + 1);

        // add new services from dir
        config.load(dir_path).unwrap();
        assert!(config.len() == len + 2);

        // verify gentoo connection was overridden
        let c = config.get("gentoo").unwrap().as_bugzilla().unwrap();
        assert_eq!(c.auth.user.as_deref().unwrap(), "user@email.com");
    }
}
