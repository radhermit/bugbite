use std::fs;

use predicates::prelude::*;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_PATH.join("get/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("get/single-bug.expected")).unwrap();

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
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_PATH.join("errors/nonexistent-bug.json"))
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
