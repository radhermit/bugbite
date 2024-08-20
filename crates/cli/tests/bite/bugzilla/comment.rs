use std::fs;

use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla comment")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
        .failure();
}

#[tokio::test]
async fn nonexistent() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/nonexistent.json"))
        .await;

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn description() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/description.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/description")).unwrap();

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/single-bug")).unwrap();

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull id from stdin
    cmd("bite bugzilla comment -")
        .write_stdin("1\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/multiple-bugs")).unwrap();

    cmd("bite bugzilla comment 1 2")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull ids from stdin
    cmd("bite bugzilla comment -")
        .write_stdin("1\n2\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}
