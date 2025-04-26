use std::time::Duration;

use predicates::prelude::*;
use wiremock::{ResponseTemplate, matchers};

use super::*;

#[test]
fn aliases() {
    for subcmd in ["g", "get"] {
        for opt in ["-h", "--help"] {
            cmd("bite redmine")
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
    cmd("bite redmine get")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn timeout() {
    let server = start_server().await;
    let delay = Duration::from_secs(1);
    let template = ResponseTemplate::new(200).set_delay(delay);
    server.respond_custom(matchers::any(), template).await;

    cmd("bite redmine -t 0.25 get 1")
        .assert()
        .stdout("")
        .stderr("Error: request timed out\n")
        .failure();
}

#[tokio::test]
async fn single() {
    let server = start_server().await;
    server.respond(200, TEST_DATA.join("get/single.json")).await;

    let expected = indoc::indoc! {"
        ==========================================================================================
        Subject      : subject
        Reporter     : john (John Smith)
        Status       : Open
        Tracker      : Bug
        Priority     : Normal
        Created      : 2024-02-15 15:56:49 UTC
        Updated      : 2024-02-15 16:00:26 UTC
        ID           : 1
        Custom field 4 : value
        Custom field 5 : value
        Comments     : 2

        Description by john (John Smith), 2024-02-15 15:56:49 UTC
        ------------------------------------------------------------------------------------------
        description

        Comment #1 by susan (Susan Miller), 2024-02-15 16:00:26 UTC
        ------------------------------------------------------------------------------------------
        comment
    "};

    // pull ID from stdin
    cmd("bite redmine get -")
        .write_stdin("1\n")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    // default output with all extra data
    cmd("bite redmine get 1")
        .assert()
        .stdout(predicate::str::diff(expected))
        .stderr("")
        .success();

    // without comments
    for opt in ["-C", "--no-comments"] {
        cmd("bite redmine get 1")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                ==========================================================================================
                Subject      : subject
                Reporter     : john (John Smith)
                Status       : Open
                Tracker      : Bug
                Priority     : Normal
                Created      : 2024-02-15 15:56:49 UTC
                Updated      : 2024-02-15 16:00:26 UTC
                ID           : 1
                Custom field 4 : value
                Custom field 5 : value
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn browser() {
    let server = start_server().await;
    server.respond(200, TEST_DATA.join("get/single.json")).await;

    for opt in ["-b", "--browser"] {
        cmd("bite redmine get 1")
            .arg(opt)
            .env("BROWSER", "true")
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}
