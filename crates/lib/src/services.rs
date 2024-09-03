use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::service::Config;

/// Bundled service data.
static SERVICES_DATA: &str = include_str!("../../../data/services.toml");

/// Bundled service name to config mapping.
pub static SERVICES: Lazy<IndexMap<String, Config>> =
    Lazy::new(|| toml::from_str(SERVICES_DATA).unwrap());
