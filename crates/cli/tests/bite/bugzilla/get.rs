use std::fs;

use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[test]
fn aliases() {
    for subcmd in ["g", "get"] {
        for opt in ["-h", "--help"] {
            cmd("bite")
                .arg(subcmd)
                .arg(opt)
                .assert()
                .stdout(predicate::str::is_empty().not())
                .stderr("")
                .success();
        }
    }
}

#[test]
fn invalid_ids() {
    cmd("bite get")
        .arg("id")
        .assert()
        .stdout("")
        .stderr(contains("error: invalid value 'id' for '<IDS>...': "))
        .failure()
        .code(2);
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_PATH.join("get/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("get/single-bug.expected")).unwrap();

    cmd("bite get")
        .arg("12345")
        .args(["-A", "no", "-C", "no", "-H", "no"])
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_PATH.join("errors/nonexistent-bug.json"))
        .await;

    cmd("bite get")
        .arg("1")
        .assert()
        .stdout("")
        .stderr("bite: error: bugzilla: Bug #1 does not exist.\n")
        .failure();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server().await;

    server
        .respond(200, TEST_PATH.join("get/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("get/multiple-bugs.expected")).unwrap();

    cmd("bite get")
        .args(["12345", "23456", "34567"])
        .args(["-A", "no", "-C", "no", "-H", "no"])
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}
