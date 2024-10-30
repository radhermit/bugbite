use std::sync::LazyLock;

use crate::service::bugzilla::Bugzilla;

pub const BASE: &str = "http://127.0.0.1:8080/";
pub const USER: &str = "bugbite@bugbite.test";
pub const PASSWORD: &str = "bugbite";

pub static SERVICE: LazyLock<Bugzilla> = LazyLock::new(|| {
    Bugzilla::builder(BASE)
        .unwrap()
        .user(USER)
        .password(PASSWORD)
        .build()
        .unwrap()
});
