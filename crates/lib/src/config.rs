use camino::Utf8Path;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::Error;
use crate::service::{self, ClientParameters, ServiceKind};
use crate::traits::{Merge, WebClient};
use crate::utils::config_dir;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!(concat!(env!("OUT_DIR"), "/services.toml"));

/// Bugbite config support.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    /// Default connection.
    pub default_connection: Option<String>,

    /// Default client parameters.
    #[serde(flatten)]
    pub client: ClientParameters,

    /// Registered service connections.
    #[serde(skip)]
    pub services: IndexMap<String, service::Config>,
}

impl Config {
    /// Create a new Config.
    pub fn new() -> crate::Result<Self> {
        let mut builder = config::Config::builder();

        let config_dir = config_dir()?;
        let load_config = config_dir != "false";

        // load user config if it exists
        let path = config_dir.join("bugbite.toml");
        if load_config {
            builder = builder.add_source(config::File::from(path.as_ref()).required(false));
        }

        // load settings from environment
        builder = builder.add_source(config::Environment::with_prefix("BUGBITE").try_parsing(true));

        // convert settings into config
        let mut config: Config = builder
            .build()
            .and_then(|c| c.try_deserialize())
            .map_err(|e| Error::Config(format!("failed loading bugbite config: {e}")))?;

        // load bundled services
        config.services = toml::from_str(SERVICES_DATA)
            .unwrap_or_else(|e| panic!("failed loading bundled service data: {e}"));

        // load custom user services
        if load_config {
            let services_dir = config_dir.join("services");
            if services_dir.exists() {
                config.load(services_dir)?;
            }
        }

        Ok(config)
    }

    /// Add a connection config from a given path.
    fn add_config(&mut self, path: &Utf8Path) -> crate::Result<()> {
        // load service config to determine connection name
        let config: service::Config = config::Config::builder()
            .add_source(config::File::from(path.as_ref()).required(true))
            .build()
            .and_then(|c| c.try_deserialize())
            .map_err(|e| Error::Config(e.to_string()))?;

        let connection_name = config.name().trim().to_string();
        if connection_name.is_empty() {
            return Err(Error::InvalidValue(format!(
                "invalid connection name: {path}"
            )));
        }

        self.services.insert(connection_name, config);

        Ok(())
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

    pub(crate) fn get_kind(
        &self,
        kind: ServiceKind,
        connection: Option<&str>,
    ) -> crate::Result<service::Config> {
        let connection = connection
            .or(self.default_connection.as_deref())
            .ok_or_else(|| Error::InvalidValue("no connection specified".to_string()))?;

        // get default connection config
        let default_config = if let Some(config) = self.services.get(connection).cloned() {
            if config.kind() != kind {
                return Err(Error::InvalidValue(format!(
                    "invalid service type: {}",
                    config.kind()
                )));
            }
            config
        } else if Url::parse(connection).is_ok() {
            service::Config::new(kind, connection)?
        } else {
            return Err(Error::InvalidValue(format!(
                "unknown connection: {connection}"
            )));
        };

        // load default connection settings
        let default_source = config::Config::try_from(&default_config).map_err(|e| {
            Error::Config(format!(
                "failed loading default service config: {connection}: {e}"
            ))
        })?;
        let mut builder = config::Config::builder().add_source(default_source);

        // load custom user service options from non-connection specific env vars
        builder = builder.add_source(config::Environment::with_prefix("BUGBITE").try_parsing(true));

        // load custom user service options from connection specific env vars
        let env_prefix = format!("BUGBITE_{}", connection.to_uppercase());
        builder = builder.add_source(
            config::Environment::with_prefix(&env_prefix)
                .try_parsing(true)
                .separator("__"),
        );

        let mut config: service::Config = builder
            .build()
            .and_then(|c| c.try_deserialize())
            .map_err(|e| {
                Error::Config(format!("failed loading service config: {connection}: {e}"))
            })?;

        // merge default client parameters
        config.merge(self.client.clone());

        Ok(config)
    }

    /// Get the service variant for the default connection if it exists.
    pub fn default_service(&self) -> Option<ServiceKind> {
        self.default_connection
            .as_ref()
            .and_then(|connection| self.services.get(connection))
            .map(|config| config.kind())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use tempfile::tempdir;

    use crate::service::bugzilla::Bugzilla;

    use super::*;

    #[test]
    fn load() {
        // ignore system user config
        unsafe { env::set_var("BUGBITE_CONFIG_DIR", "false") };

        let mut config = Config::new().unwrap();
        assert!(!config.services.is_empty());
        let len = config.services.len();

        // verify bundled gentoo connection doesn't set a user
        let service = Bugzilla::config_builder(&config, Some("gentoo"))
            .unwrap()
            .build()
            .unwrap();
        assert!(service.config().auth.user.is_none());

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
        assert!(config.services.len() == len + 1);

        // add new services from dir
        config.load(dir_path).unwrap();
        assert!(config.services.len() == len + 2);

        // verify gentoo connection was overridden
        let service = Bugzilla::config_builder(&config, Some("gentoo"))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(
            service.config().auth.user.as_deref().unwrap(),
            "user@email.com"
        );
    }
}
