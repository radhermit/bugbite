use std::fs;

use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[test]
fn aliases() {
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
    for opts in [vec![], vec!["-f", "id"], vec!["-S", "id"]] {
        cmd("bite search")
            .args(opts)
            .assert()
            .stdout("")
            .stderr("bite: error: no search terms specified\n")
            .failure()
            .code(2);
    }
}

#[test]
fn invalid_ids() {
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

    for opt in ["-f", "--fields"] {
        cmd("bite search")
            .args([opt, "id", "test"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }
}
