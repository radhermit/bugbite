use std::fs;

use bugbite::test::TESTDATA_PATH;
use predicates::prelude::*;

use crate::command::cmd;

use super::start_server;

#[tokio::test]
async fn get() {
    let server = start_server().await;
    let path = TESTDATA_PATH.join("bugzilla/get");

    // single bug
    server.respond(200, path.join("single-bug.json")).await;
    let expected = fs::read_to_string(path.join("single-bug.expected")).unwrap();

    for subcmd in ["g", "get"] {
        cmd("bite")
            .arg(subcmd)
            .arg("12345")
            .args(["-A", "no", "-C", "no", "-H", "no"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }

    server.reset().await;

    // nonexistent bug
    server
        .respond(404, path.join("error-nonexistent-bug.json"))
        .await;

    for subcmd in ["g", "get"] {
        cmd("bite")
            .arg(subcmd)
            .arg("1")
            .assert()
            .stdout("")
            .stderr("bite: error: bugzilla: Bug #1 does not exist.\n")
            .failure();
    }
}
