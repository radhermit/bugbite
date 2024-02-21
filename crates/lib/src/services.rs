use indexmap::IndexMap;
use once_cell::sync::Lazy;

use crate::service::{Config, ServiceKind};

/// Pre-defined services.
#[rustfmt::skip]
pub static SERVICES: Lazy<IndexMap<String, Config>> = Lazy::new(|| {
    use ServiceKind::*;
    [
        (BugzillaRestV1, "gentoo", "https://bugs.gentoo.org/"),
        (BugzillaRestV1, "gcc", "https://gcc.gnu.org/bugzilla/"),
        (BugzillaRestV1, "glibc", "https://sourceware.org/bugzilla/"),
        (BugzillaRestV1, "kde", "https://bugs.kde.org/"),
        (BugzillaRestV1, "linux", "https://bugzilla.kernel.org/"),
        (BugzillaRestV1, "libreoffice", "https://bugs.documentfoundation.org/"),
        (BugzillaRestV1, "mozilla", "https://bugzilla.mozilla.org/"),
        (BugzillaRestV1, "redhat", "https://bugzilla.redhat.com/"),
        (Github, "bugbite", "https://github.com/radhermit/bugbite/"),
    ]
    .into_iter()
    .map(|(kind, name, base)| {
        let service = kind.create(base).unwrap_or_else(|e| panic!("{e}"));
        (name.to_string(), service)
    })
    .collect()
});
