use std::fs;

use predicates::prelude::*;
use wiremock::matchers;

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
fn required_args() {
    // missing IDs
    cmd("bite bugzilla get")
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

    cmd("bite bugzilla get 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
        .failure();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;
    server
        .respond_match(
            matchers::path("/rest/bug/1"),
            200,
            TEST_DATA.join("get/single-bug.json"),
        )
        .await;
    server
        .respond_match(
            matchers::path("/rest/bug/1/attachment"),
            200,
            TEST_DATA.join("attachment/get/bug-with-attachments.json"),
        )
        .await;
    server
        .respond_match(
            matchers::path("/rest/bug/1/comment"),
            200,
            TEST_DATA.join("comment/single-bug.json"),
        )
        .await;
    server
        .respond_match(
            matchers::path("/rest/bug/1/history"),
            200,
            TEST_DATA.join("history/single-bug.json"),
        )
        .await;

    let expected = indoc::indoc! {"
        ==========================================================================================
        Summary      : new summary
        Assignee     : assignee
        Creator      : person
        Created      : 2024-03-13 14:02:53 UTC
        Updated      : 2024-03-15 22:31:48 UTC
        Status       : CONFIRMED
        Whiteboard   : whiteboard
        Component    : component
        Product      : product
        Platform     : All
        OS           : Linux
        Priority     : High
        Severity     : normal
        ID           : 1
        Alias        : alias
        CC           : person1, person2
        See also     :
          https://github.com/radhermit/bugbite/issues/1
          https://github.com/radhermit/bugbite/issues/2
    "};

    // bug fields only, no extra data
    cmd("bite bugzilla get -ACH 1")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    // pull ID from stdin
    cmd("bite bugzilla get -ACH -")
        .write_stdin("1\n")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    let expected = fs::read_to_string(TEST_OUTPUT.join("get/single-bug-default")).unwrap();

    // default output with all extra data
    cmd("bite bugzilla get 1")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    let expected = fs::read_to_string(TEST_OUTPUT.join("get/single-bug-attachments")).unwrap();

    // bug fields with attachments
    cmd("bite bugzilla get -CH 1")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    let expected = fs::read_to_string(TEST_OUTPUT.join("get/single-bug-no-attachments")).unwrap();

    // bug fields without attachments
    cmd("bite bugzilla get -A 1")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("get/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("get/multiple-bugs")).unwrap();

    cmd("bite bugzilla get -ACH 12345 23456 34567")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull IDs from stdin
    cmd("bite bugzilla get -ACH -")
        .write_stdin("12345\n23456\n34567\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn browser() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("get/single-bug.json"))
        .await;

    for opt in ["-b", "--browser"] {
        cmd("bite bugzilla get 1")
            .arg(opt)
            .env("BROWSER", "true")
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}
