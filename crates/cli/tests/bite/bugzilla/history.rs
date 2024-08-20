use std::fs;

use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla history")
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

    cmd("bite bugzilla history 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
        .failure();
}

#[tokio::test]
async fn nonexistent() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("history/nonexistent.json"))
        .await;

    cmd("bite bugzilla history 1")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("history/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("history/single-bug")).unwrap();

    cmd("bite bugzilla history 1")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull id from stdin
    cmd("bite bugzilla history -")
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
        .respond(200, TEST_DATA.join("history/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("history/multiple-bugs")).unwrap();

    cmd("bite bugzilla history 1 2")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull ids from stdin
    cmd("bite bugzilla history -")
        .write_stdin("1\n2\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn creator() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("history/single-bug.json"))
        .await;

    for opt in ["-R", "--creator"] {
        cmd("bite bugzilla history 1")
            .args([opt, "user1"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                Bug: 1 ===================================================================================
                Changes made by user1@bugbite.test, 2024-03-13 14:22:39 UTC
                ------------------------------------------------------------------------------------------
                summary: old summary -> new summary

                Changes made by user1@bugbite.test, 2024-03-15 03:11:09 UTC
                ------------------------------------------------------------------------------------------
                blocks: -100
            "}))
            .stderr("")
            .success();
    }
}
