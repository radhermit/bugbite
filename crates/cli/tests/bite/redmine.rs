use std::env;

use bugbite::test::TestServer;
use predicates::str::contains;

use crate::command::cmd;

mod get;

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    let base = server.uri();
    env::set_var("BUGBITE_CONNECTION", format!("{base}/projects/test"));
    server
}

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite redmine")
            .args([opt, "gentoo"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("incompatible connection: gentoo"))
            .failure();
    }
}

#[test]
fn unknown_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite redmine")
            .args([opt, "unknown"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("unknown connection: unknown"))
            .failure();
    }
}
