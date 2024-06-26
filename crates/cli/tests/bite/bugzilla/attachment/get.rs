use std::{env, fs};

use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use crate::bugzilla::*;

#[test]
fn aliases() {
    for subcmd in ["g", "get"] {
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
fn missing_ids() {
    cmd("bite bugzilla attachment get")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd("bite bugzilla attachment get")
            .args([opt, "1"])
            .assert()
            .stdout("")
            .stderr("Error: bugzilla: Bug #1 does not exist.\n")
            .failure();
    }
}

#[tokio::test]
async fn deleted_attachment() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("attachment/deleted.json"))
        .await;

    cmd("bite bugzilla attachment get 21")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: can't retrieve deleted attachment: 21").trim())
        .failure();
}

#[tokio::test]
async fn list_single_without_data() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("attachment/single-without-data.json"))
        .await;

    let expected = fs::read_to_string(TEST_OUTPUT.join("attachment/single-without-data")).unwrap();

    // default output
    for opt in ["-l", "--list"] {
        cmd("bite bugzilla attachment get")
            .arg("123")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }

    // verbose output
    for opt in ["-l", "--list"] {
        cmd("bite bugzilla attachment get -v")
            .arg("123")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr(predicate::str::is_empty().not())
            .success();
    }
}

#[tokio::test]
async fn output_stdout_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/single-plain-text.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("attachment/single-plain-text")).unwrap();

    for opt in ["-o", "--output"] {
        cmd("bite bugzilla attachment get")
            .arg("123")
            .args([opt, "-"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn save_single_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/single-plain-text.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("attachment/single-plain-text")).unwrap();

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();
    // save files to the current working directory
    env::set_current_dir(dir_path).unwrap();

    cmd("bite bugzilla attachment get")
        .arg("123")
        .assert()
        .stdout(predicate::str::diff("Saving attachment: ./test.txt\n"))
        .stderr("")
        .success();

    // verify file content
    let file = dir.path().join("test.txt");
    let data = fs::read_to_string(file).unwrap();
    assert_eq!(&data, &expected);
}

#[tokio::test]
async fn save_single_existing_error() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/single-plain-text.json"))
        .await;

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    cmd("bite bugzilla attachment get")
        .arg("123")
        .args(["-d", dir_path])
        .assert()
        .stdout(predicate::str::diff(format!(
            "Saving attachment: {dir_path}/test.txt\n"
        )))
        .stderr("")
        .success();

    // re-running causes a file existence failure
    cmd("bite bugzilla attachment get")
        .arg("123")
        .args(["-d", dir_path])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(format!(
            "Error: file already exists: {dir_path}/test.txt\n"
        )))
        .failure();
}

#[tokio::test]
async fn single_bug_with_no_attachments() {
    let server = start_server().await;

    server
        .respond(
            200,
            TEST_DATA.join("attachment/bug-with-no-attachments.json"),
        )
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd("bite bugzilla attachment get")
            .args([opt, "12345"])
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn multiple_bugs_with_no_attachments() {
    let server = start_server().await;

    server
        .respond(
            200,
            TEST_DATA.join("attachment/bugs-with-no-attachments.json"),
        )
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd("bite bugzilla attachment get")
            .args([opt, "12345", "23456", "34567"])
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn save_multiple_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/bugs-with-attachments.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("attachment/single-plain-text")).unwrap();

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();
    // save files to the current working directory
    env::set_current_dir(dir_path).unwrap();

    let ids = ["12345", "23456", "34567"];
    cmd("bite bugzilla attachment get -i")
        .args(ids)
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();

    // verify file content
    for id in ids {
        let file = dir.path().join(format!("{id}/test.txt"));
        let data = fs::read_to_string(file).unwrap();
        assert_eq!(&data, &expected);
    }
}
