use std::env;

use bugbite::test::bugzilla::*;

mod command;
mod create;
mod search;
mod update;

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
