use std::fs;

use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["g", "get"] {
        for opt in ["-h", "--help"] {
            cmd("bite bugzilla")
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
fn missing_ids() {
    cmd("bite bugzilla get")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("get/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("get/single-bug")).unwrap();

    cmd("bite bugzilla get -ACH")
        .arg("12345")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull id from stdin
    cmd("bite bugzilla get -ACH -")
        .write_stdin("12345\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    cmd("bite bugzilla get")
        .arg("1")
        .assert()
        .stdout("")
        .stderr("Error: bugzilla: Bug #1 does not exist.\n")
        .failure();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("get/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("get/multiple-bugs")).unwrap();

    cmd("bite bugzilla get -ACH")
        .args(["12345", "23456", "34567"])
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull ids from stdin
    cmd("bite bugzilla get -ACH -")
        .write_stdin("12345\n23456\n34567\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}
