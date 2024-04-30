use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["u", "update"] {
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
fn missing_ids() {
    cmd("bite bugzilla update -A test")
        .assert()
        .stdout("")
        .stderr(contains("required arguments were not provided"))
        .failure()
        .code(2);
}

#[tokio::test]
async fn auth_required() {
    let _server = start_server().await;

    cmd("bite update 1 -A test")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}
