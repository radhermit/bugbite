use std::{env, fs};

use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[test]
fn aliases() {
    env::set_var("BUGBITE_BASE", "fake://bugbite");
    for subcmd in ["s", "search"] {
        for opt in ["-h", "--help"] {
            cmd("bite")
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
fn no_search_terms() {
    env::set_var("BUGBITE_BASE", "fake://bugbite");
    cmd("bite search")
        .assert()
        .stdout("")
        .stderr("bite: error: no search terms specified\n")
        .failure()
        .code(2);
}

#[test]
fn invalid_ids() {
    env::set_var("BUGBITE_BASE", "fake://bugbite");
    cmd("bite search")
        .args(["--id", "id"])
        .assert()
        .stdout("")
        .stderr(contains("error: invalid value 'id' for '--id <ID>': "))
        .failure()
        .code(2);
}

#[test]
fn multiple_stdin() {
    env::set_var("BUGBITE_BASE", "fake://bugbite");
    cmd("bite search --id - -")
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

    server.respond(200, TEST_PATH.join("search/ids.json")).await;
    let expected = fs::read_to_string(TEST_PATH.join("search/ids.expected")).unwrap();

    cmd("bite search")
        .args(["-F", "id", "test"])
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}
