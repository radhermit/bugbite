use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["c", "create"] {
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
async fn creation() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("create/creation.json"))
        .await;

    cmd("bite bugzilla create")
        .args(["--component", "TestComponent"])
        .args(["--product", "TestProduct"])
        .args(["--summary", "test"])
        .args(["--description", "test"])
        .assert()
        .stdout("123\n")
        .stderr("")
        .success();
}
