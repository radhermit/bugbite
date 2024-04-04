use std::fs;
use std::time::Duration;

use predicates::prelude::*;
use wiremock::{matchers, ResponseTemplate};

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
async fn timeout() {
    let server = start_server().await;
    let delay = Duration::from_secs(1);
    let template = ResponseTemplate::new(200).set_delay(delay);
    server.respond_custom(matchers::any(), template).await;

    cmd("bite -t 0.25 get 1")
        .assert()
        .stdout("")
        .stderr("Error: request timed out\n")
        .failure();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("get/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("get/single-bug")).unwrap();

    cmd("bite get -ACH")
        .arg("12345")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull id from stdin
    cmd("bite get -ACH -")
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

    cmd("bite get")
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

    cmd("bite get -ACH")
        .args(["12345", "23456", "34567"])
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull ids from stdin
    cmd("bite get -ACH -")
        .write_stdin("12345\n23456\n34567\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}
