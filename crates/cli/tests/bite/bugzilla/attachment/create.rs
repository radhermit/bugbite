use std::{env, fs};

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

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla attachment create")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);

    // missing files
    cmd("bite bugzilla attachment create 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
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
            "Error: failed reading attachment: 2",
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
    env::set_current_dir(dir.path()).unwrap();
    fs::create_dir("src").unwrap();

    // invalid MIME type
    cmd("bite bugzilla attachment create 1 src")
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
    cmd("bite bugzilla attachment create 1 src")
        .arg("--patch")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: patch type invalid for directory targets").trim())
        .failure()
        .code(1);

    // empty directory target
    cmd("bite bugzilla attachment create 1 src")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: empty directory target: src").trim())
        .failure()
        .code(1);

    // create files
    fs::write("src/test1", "test1").unwrap();
    fs::write("src/test2", "test2").unwrap();

    // valid
    cmd("bite bugzilla attachment create 1 src")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
