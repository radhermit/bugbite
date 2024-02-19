use std::fs;

use bugbite::test::TESTDATA_PATH;
use predicates::prelude::*;

use crate::command::cmd;

use super::start_server;

#[tokio::test]
async fn search() {
    let server = start_server().await;
    let path = TESTDATA_PATH.join("bugzilla/search");

    // only ids
    server.respond(200, path.join("ids.json")).await;
    let expected = fs::read_to_string(path.join("ids.expected")).unwrap();

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
