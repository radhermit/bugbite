use std::{env, fs};

use bugbite::test::TestServer;
use predicates::prelude::*;

use crate::command::cmd;
use crate::macros::build_path;

#[tokio::test]
async fn search() {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");
    let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");

    // only ids
    server.respond(200, "bugzilla/search/ids.json").await;
    let expected = fs::read_to_string(path.join("bugzilla/search/ids.expected")).unwrap();

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
