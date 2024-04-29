use std::env;

use bugbite::test::TestServer;
use predicates::str::contains;

use crate::command::cmd;

mod get;

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    let base = server.uri();
    env::set_var("BUGBITE_BASE", format!("{base}/projects/test"));
    env::set_var("BUGBITE_SERVICE", "redmine");
    server
}

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite")
            .args([opt, "gentoo"])
            .arg("redmine")
            .assert()
            .stdout("")
            .stderr(contains("redmine not compatible with connection: gentoo"))
            .failure();
    }
}

#[test]
fn no_connection() {
    for action in ["s", "search"] {
        cmd("bite redmine")
            .args([action, "-c", "1d"])
            .assert()
            .stdout("")
            .stderr(contains("no connection specified"))
            .failure();
    }
}
