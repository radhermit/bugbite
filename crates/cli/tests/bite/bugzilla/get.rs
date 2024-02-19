use std::{env, fs};

use bugbite::test::TestServer;
use predicates::prelude::*;

use crate::command::cmd;
use crate::macros::build_path;

#[tokio::test]
async fn single_bug() {
    let server = TestServer::new().await;
    server.respond(200, "bugzilla/get/single-bug.json").await;
    let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");
    let expected = fs::read_to_string(path.join("bugzilla/get/single-bug")).unwrap();
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");

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
