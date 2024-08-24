use std::env;

use bugbite::test::{build_path, TestServer};
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;

use command::cmd;

mod bugzilla;
mod command;
mod redmine;
mod show;

pub(crate) static TEST_DATA_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_CONNECTION", server.uri());
    server
}

async fn start_server_with_auth() -> TestServer {
    let server = start_server().await;
    env::set_var("BUGBITE_USER", "bugbite@bugbite.test");
    env::set_var("BUGBITE_PASS", "bugbite");
    env::set_var("BUGBITE_KEY", "bugbite");
    server
}

/// Initialization for all test executables.
#[ctor::ctor]
fn initialize() {
    // wipe bugbite-related environment variables
    for (key, _value) in env::vars() {
        if key.starts_with("BUGBITE_") {
            env::remove_var(key);
        }
    }
}

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn version() {
    let version = env!("CARGO_PKG_VERSION");
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(format!("bite {version}")).trim())
            .stderr("")
            .success();
    }
}
