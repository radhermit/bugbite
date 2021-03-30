use crate::service::ServiceKind;

/// Connection config support
#[derive(Debug, Default)]
pub struct Config {
    base: String,
    service: ServiceKind,
}

impl Config {
    pub fn load_path(path: &str) -> crate::Result<Self> {
        Ok(Config::default())
    }

    pub fn load() -> crate::Result<Self> {
        Ok(Config::default())
    }
}
