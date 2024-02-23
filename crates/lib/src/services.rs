use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::service::{Config, ServiceKind};

/// Pre-defined services.
#[rustfmt::skip]
pub static SERVICES: Lazy<IndexMap<String, Config>> = Lazy::new(|| {
    use ServiceKind::*;
    [
        (Bugzilla, "gentoo", "https://bugs.gentoo.org/"),
        (Bugzilla, "gcc", "https://gcc.gnu.org/bugzilla/"),
        (Bugzilla, "glibc", "https://sourceware.org/bugzilla/"),
        (Bugzilla, "kde", "https://bugs.kde.org/"),
        (Bugzilla, "linux", "https://bugzilla.kernel.org/"),
        (Bugzilla, "libreoffice", "https://bugs.documentfoundation.org/"),
        (Bugzilla, "mozilla", "https://bugzilla.mozilla.org/"),
        (Bugzilla, "redhat", "https://bugzilla.redhat.com/"),
        (Github, "bugbite", "https://github.com/radhermit/bugbite/"),
        (Redmine, "ceph", "https://tracker.ceph.com/projects/ceph/"),
        (Redmine, "pfsense", "https://redmine.pfsense.org/projects/pfsense/"),
        (Redmine, "ruby", "https://bugs.ruby-lang.org/projects/ruby-master/"),
    ]
    .into_iter()
    .map(|(kind, name, base)| {
        let service = Config::new(kind, base).unwrap_or_else(|e| panic!("{e}"));
        (name.to_string(), service)
    })
    .collect()
});
