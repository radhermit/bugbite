use predicates::prelude::*;

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

#[tokio::test]
async fn required_args() {
    let _server = start_server().await;

    // missing IDs
    cmd("bite bugzilla update -A test")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);

    // missing changes
    cmd("bite bugzilla update 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: no parameters specified").trim())
        .failure()
        .code(1);
}

#[tokio::test]
async fn auth_required() {
    let _server = start_server().await;

    cmd("bite bugzilla update 1 -A test")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}
