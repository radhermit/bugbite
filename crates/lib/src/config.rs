use std::env;
use std::ops::Deref;

use camino::Utf8Path;
use indexmap::IndexMap;
use url::Url;

use crate::service::{self, ServiceKind};
use crate::traits::WebClient;
use crate::utils::config_dir;
use crate::Error;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!(concat!(env!("OUT_DIR"), "/services.toml"));

/// Connection config support.
#[derive(Debug, Default)]
pub struct Config {
    services: IndexMap<String, service::Config>,
}

impl Config {
    /// Create a new Config.
    pub fn new() -> crate::Result<Self> {
        let mut config = Config {
            services: toml::from_str(SERVICES_DATA)
                .unwrap_or_else(|e| panic!("failed loading bundled service data: {e}")),
        };

        // load custom user services
        let services_dir = config_dir()?.join("services");
        match env::var("BUGBITE_CONFIG").as_deref() {
            Err(_) if services_dir.exists() => config.load(services_dir)?,
            Ok("false") | Err(_) => (),
            Ok(path) => config.load(path)?,
        }

        Ok(config)
    }

    /// Add a connection config from a given path.
    fn add_config(&mut self, path: &Utf8Path) -> crate::Result<()> {
        let config = service::Config::try_from_path(path)?;
        if config.name().trim().is_empty() {
            Err(Error::InvalidValue(format!(
                "invalid connection name: {path}"
            )))
        } else {
            self.services.insert(config.name().to_string(), config);
            Ok(())
        }
    }

    /// Load connections from a given path, overriding any bundled matches.
    pub fn load<P: AsRef<Utf8Path>>(&mut self, path: P) -> crate::Result<()> {
        let path = path.as_ref();
        if path.is_dir() {
            for entry in path.read_dir_utf8()? {
                let entry = entry?;
                self.add_config(entry.path())?;
            }
        } else {
            self.add_config(path)?;
        }

        // re-sort by connection name
        self.services.sort_keys();

        Ok(())
    }

    pub fn get_kind(&self, kind: ServiceKind, name: &str) -> crate::Result<service::Config> {
        if let Some(config) = self.services.get(name).cloned() {
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
        &self.services
    }
}

impl IntoIterator for Config {
    type Item = (String, service::Config);
    type IntoIter = indexmap::map::IntoIter<String, service::Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.services.into_iter()
    }
}

impl<'a> IntoIterator for &'a Config {
    type Item = (&'a String, &'a service::Config);
    type IntoIter = indexmap::map::Iter<'a, String, service::Config>;

    fn into_iter(self) -> Self::IntoIter {
        self.services.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn load() {
        // ignore custom user services
        env::set_var("BUGBITE_CONFIG", "false");

        let mut config = Config::new().unwrap();
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
            type = "bugzilla"
            name = "new1"
            base = "https://random.bugzilla.site/"
        "#};
        fs::write("1.toml", service1).unwrap();
        let service2 = indoc::indoc! {r#"
            type = "redmine"
            name = "new2"
            base = "https://random.redmine.site/"
        "#};
        fs::write("2.toml", service2).unwrap();
        let gentoo = indoc::indoc! {r#"
            type = "bugzilla"
            name = "gentoo"
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
