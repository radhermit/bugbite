use std::env;

use bugbite::test::TestServer;
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::str::contains;

use crate::command::cmd;

mod attachment;
mod comment;
mod create;
mod get;
mod history;
mod search;
mod update;

static TEST_DATA: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("bugbite/bugzilla"));
static TEST_OUTPUT: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("output/bugzilla"));

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla");
    server
}

async fn start_server_with_auth() -> TestServer {
    let server = start_server().await;
    env::set_var("BUGBITE_USER", "bugbite@bugbite.test");
    env::set_var("BUGBITE_PASS", "bugbite");
    server
}

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite")
            .args([opt, "ruby"])
            .arg("bugzilla")
            .assert()
            .stdout("")
            .stderr(contains("bugzilla not compatible with connection: ruby"))
            .failure();
    }
}

#[test]
fn no_connection() {
    for action in ["s", "search"] {
        cmd("bite bugzilla")
            .args([action, "-c", "1d"])
            .assert()
            .stdout("")
            .stderr(contains("no connection specified"))
            .failure();
    }
}
