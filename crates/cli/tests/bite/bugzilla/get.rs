use std::{env, fs};

use bugbite::test::TestServer;
use predicates::prelude::*;

use crate::command::cmd;
use crate::macros::build_path;

#[tokio::test]
async fn get() {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");
    let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");

    // single bug
    server.respond(200, "bugzilla/get/single-bug.json").await;
    let expected = fs::read_to_string(path.join("bugzilla/get/single-bug.expected")).unwrap();

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

    server.reset().await;

    // nonexistent bug
    server
        .respond(404, "bugzilla/get/error-nonexistent-bug.json")
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
