use std::fs;

use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[test]
fn invalid_ids() {
    for subcmd in ["s", "search"] {
        cmd("bite")
            .arg(subcmd)
            .args(["--id", "id"])
            .assert()
            .stdout("")
            .stderr(contains("error: invalid value 'id' for '--id <ID>': "))
            .failure()
            .code(2);
    }
}

#[tokio::test]
async fn ids_only() {
    let server = start_server().await;

    server.respond(200, TEST_PATH.join("search/ids.json")).await;
    let expected = fs::read_to_string(TEST_PATH.join("search/ids.expected")).unwrap();

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
