use std::fs;

use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["s", "search"] {
        for opt in ["-h", "--help"] {
            cmd("bite bugzilla")
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
fn invalid_ids() {
    cmd("bite bugzilla search")
        .args(["--id", "id"])
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn multiple_stdin() {
    cmd("bite bugzilla search --id - -")
        .write_stdin("12345\n")
        .assert()
        .stdout("")
        .stderr(contains("stdin argument used more than once"))
        .failure()
        .code(2);
}

#[tokio::test]
async fn ids_only() {
    let server = start_server().await;

    server.respond(200, TEST_DATA.join("search/ids.json")).await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("search/ids")).unwrap();

    for opt in ["-f", "--fields"] {
        cmd("bite bugzilla search")
            .args([opt, "id", "test"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }
}
