use std::env;

use bugbite::service::bugzilla::{Config, Service};
use once_cell::sync::Lazy;

mod command;
mod create;
mod search;

const BASE: &str = "http://127.0.0.1:8080/";
const USER: &str = "bugbite@bugbite.test";
const PASSWORD: &str = "bugbite";

pub(crate) static SERVICE: Lazy<Service> = Lazy::new(|| {
    let mut config = Config::new(BASE).unwrap();
    config.user = Some(USER.to_string());
    config.password = Some(PASSWORD.to_string());
    Service::new(config, Default::default()).unwrap()
});

/// Initialization for all test executables.
#[ctor::ctor]
fn initialize() {
    // wipe bugbite-related environment variables
    for (key, _value) in env::vars() {
        if key.starts_with("BUGBITE_") {
            env::remove_var(key);
        }
    }

    // use local bugzilla instance
    env::set_var("BUGBITE_CONNECTION", BASE);
    env::set_var("BUGBITE_USER", USER);
    env::set_var("BUGBITE_PASS", PASSWORD);
}
