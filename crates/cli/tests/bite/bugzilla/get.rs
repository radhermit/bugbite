use std::fs;

use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;

use crate::command::cmd;

use super::start_server;

static TEST_PATH: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TESTDATA_PATH.join("bugzilla/get"));

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    // single bug
    server.respond(200, TEST_PATH.join("single-bug.json")).await;
    let expected = fs::read_to_string(TEST_PATH.join("single-bug.expected")).unwrap();

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

    // nonexistent bug
    server
        .respond(404, TEST_PATH.join("error-nonexistent-bug.json"))
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
