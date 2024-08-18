use predicates::prelude::*;
use tempfile::tempdir;

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
async fn required_args() {
    let _server = start_server().await;

    // missing fields
    let err = "Error: missing required fields: component, description, product, summary";
    cmd("bite bugzilla create")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(err).trim())
        .failure()
        .code(1);
}

#[tokio::test]
async fn auth_required() {
    let _server = start_server().await;

    cmd("bite bugzilla create")
        .args(["--component", "TestComponent"])
        .args(["--product", "TestProduct"])
        .args(["--summary", "test"])
        .args(["--description", "test"])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}

#[tokio::test]
async fn creation() {
    let server = start_server_with_auth().await;

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

#[tokio::test]
async fn templates() {
    let server = start_server_with_auth().await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

    // create template
    cmd("bite bugzilla create --dry-run")
        .args(["--component", "TestComponent"])
        .args(["--product", "TestProduct"])
        .args(["--summary", "test"])
        .args(["--description", "test"])
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    server
        .respond(200, TEST_DATA.join("create/creation.json"))
        .await;

    // create bug from template
    cmd("bite bugzilla create")
        .args(["--from", path])
        .assert()
        .stdout("123\n")
        .stderr("")
        .success();
}
