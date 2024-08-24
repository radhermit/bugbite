use std::env;
use std::time::Duration;

use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::str::contains;
use wiremock::{matchers, ResponseTemplate};

use crate::command::cmd;

use super::*;

mod attachment;
mod comment;
mod create;
mod get;
mod history;
mod search;
mod update;

static TEST_DATA: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("bugbite/bugzilla"));
static TEST_OUTPUT: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("output/bugzilla"));

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite bugzilla")
            .args([opt, "ruby"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("incompatible connection: ruby"))
            .failure();
    }
}

#[test]
fn unknown_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite bugzilla")
            .args([opt, "unknown"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("unknown connection: unknown"))
            .failure();
    }
}

// timeout support works at a global session level, but only a specific request is tested here
#[tokio::test]
async fn timeout() {
    let server = start_server().await;
    let delay = Duration::from_secs(1);
    let template = ResponseTemplate::new(200).set_delay(delay);
    server.respond_custom(matchers::any(), template).await;

    cmd("bite bugzilla -t 0.25 get 1")
        .assert()
        .stdout("")
        .stderr("Error: request timed out\n")
        .failure();
}
