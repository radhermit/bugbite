use std::env;

mod command;
mod create;
mod search;

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
    env::set_var("BUGBITE_BASE", "http://127.0.0.1:8080/");
    env::set_var("BUGBITE_SERVICE", "bugzilla");
    env::set_var("BUGBITE_USER", "bugbite@bugbite.test");
    env::set_var("BUGBITE_PASS", "bugbite");
}
