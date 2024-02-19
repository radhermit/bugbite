use std::fs;

use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;

use crate::command::cmd;

use super::start_server;

static TEST_PATH: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TESTDATA_PATH.join("bugzilla/search"));

#[tokio::test]
async fn ids_only() {
    let server = start_server().await;

    // only ids
    server.respond(200, TEST_PATH.join("ids.json")).await;
    let expected = fs::read_to_string(TEST_PATH.join("ids.expected")).unwrap();

    for subcmd in ["s", "search"] {
        cmd("bite")
            .arg(subcmd)
            .args(["-F", "id", "test"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }
}
