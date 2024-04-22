use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["m", "modify"] {
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
async fn auth_required() {
    let _server = start_server().await;

    cmd("bite modify 1 -A test")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}
