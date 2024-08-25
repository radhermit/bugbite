use std::time::Duration;

use predicates::prelude::*;
use wiremock::{matchers, ResponseTemplate};

use crate::command::cmd;

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
