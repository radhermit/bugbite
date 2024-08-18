use predicates::prelude::*;
use tempfile::NamedTempFile;

use crate::command::cmd;

use crate::bugzilla::*;

#[test]
fn aliases() {
    for subcmd in ["c", "create"] {
        for opt in ["-h", "--help"] {
            cmd("bite bugzilla attachment")
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
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("attachment/create/single.json"))
        .await;

    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("attachment/create/multiple.json"))
        .await;

    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    // invalid command -- ID args must be in a single comma-separated string
    cmd("bite bugzilla attachment create 1 2")
        .arg(path)
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "Error: invalid attachment source: 2",
        ))
        .failure()
        .code(1);

    cmd("bite bugzilla attachment create 1,2")
        .arg(path)
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
