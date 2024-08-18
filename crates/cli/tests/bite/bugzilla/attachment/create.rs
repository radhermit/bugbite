use std::fs;

use predicates::prelude::*;
use tempfile::{tempdir, NamedTempFile};

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

#[tokio::test]
async fn dir_target() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("attachment/create/single.json"))
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("src");
    let path = path.to_str().unwrap();
    fs::create_dir(path).unwrap();

    // invalid MIME type
    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .args(["--mime", "text/plain"])
        .assert()
        .stdout("")
        .stderr(
            predicate::str::diff("Error: MIME type invalid for directory targets: text/plain")
                .trim(),
        )
        .failure()
        .code(1);

    // invalid MIME type
    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .arg("--patch")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: patch type invalid for directory targets").trim())
        .failure()
        .code(1);

    // empty directory target
    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("Error: empty directory target"))
        .failure()
        .code(1);

    // create files
    fs::write(dir.path().join("src/test1"), "test1").unwrap();
    fs::write(dir.path().join("src/test2"), "test2").unwrap();

    // valid
    cmd("bite bugzilla attachment create 1")
        .arg(path)
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
