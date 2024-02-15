use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::service::{Config, ServiceKind};
use crate::Error;

#[derive(Debug, Clone)]
pub struct Services {
    services: IndexMap<String, Config>,
}

impl Services {
    pub fn get(&self, name: &str) -> crate::Result<&Config> {
        self.services
            .get(name)
            .ok_or_else(|| Error::InvalidValue(format!("unknown service: {name}")))
    }

    pub fn get_raw(&self, name: &str) -> crate::Result<(ServiceKind, String)> {
        let service = self
            .services
            .get(name)
            .ok_or_else(|| Error::InvalidValue(format!("unknown service: {name}")))?;
        Ok((service.kind(), service.base().to_string()))
    }

    pub fn iter(&self) -> indexmap::map::Iter<String, Config> {
        self.services.iter()
    }
}

/// Pre-defined services.
pub static SERVICES: Lazy<Services> = Lazy::new(|| {
    use ServiceKind::*;
    let services = [
        (BugzillaRestV1, "gentoo", "https://bugs.gentoo.org"),
        (BugzillaRestV1, "gcc", "https://gcc.gnu.org.bugzilla/"),
        (BugzillaRestV1, "linux", "https://bugzilla.kernel.org/"),
        (Github, "bugbite", "https://github.com/radhermit/bugbite/"),
    ]
    .into_iter()
    .map(|(kind, name, base)| {
        let service = kind.create(base).unwrap_or_else(|e| panic!("{e}"));
        (name.to_string(), service)
    })
    .collect();
    Services { services }
});
