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
    env::set_var("BUGBITE_CONNECTION", server.uri());
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
        cmd("bite bugzilla")
            .args([opt, "ruby"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("incompatible connection: ruby"))
            .failure();
    }
}

#[test]
fn unknown_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite bugzilla")
            .args([opt, "unknown"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("unknown connection: unknown"))
            .failure();
    }
}
