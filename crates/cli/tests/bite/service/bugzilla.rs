use std::env;
use std::sync::LazyLock;
use std::time::Duration;

use camino::Utf8PathBuf;
use predicates::prelude::*;
use wiremock::{ResponseTemplate, matchers};

use super::*;

mod attachment;
mod comment;
mod create;
mod fields;
mod get;
mod history;
mod search;
mod update;
mod version;

static TEST_DATA: LazyLock<Utf8PathBuf> =
    LazyLock::new(|| crate::TEST_DATA_PATH.join("bugbite/bugzilla"));
static TEST_OUTPUT: LazyLock<Utf8PathBuf> =
    LazyLock::new(|| crate::TEST_DATA_PATH.join("output/bugzilla"));

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite bugzilla")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn invalid_service_type() {
    for opt in ["-c", "--connection"] {
        cmd("bite bugzilla")
            .args([opt, "ruby"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("invalid service type: redmine"))
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
            .stderr(predicate::str::contains("unknown connection: unknown"))
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
