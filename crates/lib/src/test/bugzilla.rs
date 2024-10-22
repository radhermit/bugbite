use std::sync::LazyLock;

use crate::service::bugzilla::{Bugzilla, Config};

pub const BASE: &str = "http://127.0.0.1:8080/";
pub const USER: &str = "bugbite@bugbite.test";
pub const PASSWORD: &str = "bugbite";

pub static SERVICE: LazyLock<Bugzilla> = LazyLock::new(|| {
    let mut config = Config::new(BASE).unwrap();
    config.auth.user = Some(USER.to_string());
    config.auth.password = Some(PASSWORD.to_string());
    config.into_service().unwrap()
});
