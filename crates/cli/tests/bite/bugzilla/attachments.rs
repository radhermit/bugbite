use std::{env, fs};

use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use super::{start_server, TEST_PATH};

#[tokio::test]
async fn list_single_without_data() {
    let server = start_server().await;

    server
        .respond(200, TEST_PATH.join("attachments/single-without-data.json"))
        .await;
    let expected =
        fs::read_to_string(TEST_PATH.join("attachments/single-without-data.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opt in ["-l", "--list"] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .arg(opt)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn view_single_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_PATH.join("attachments/single-plain-text.json"))
        .await;
    let expected =
        fs::read_to_string(TEST_PATH.join("attachments/single-plain-text.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opt in ["-V", "--view"] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .arg(opt)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn save_single_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_PATH.join("attachments/single-plain-text.json"))
        .await;
    let expected =
        fs::read_to_string(TEST_PATH.join("attachments/single-plain-text.expected")).unwrap();

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();
    // save files to the current working directory
    env::set_current_dir(dir_path).unwrap();

    for subcmd in ["a", "attachments"] {
        cmd("bite")
            .arg(subcmd)
            .arg("123")
            .assert()
            .stdout(predicate::str::diff("Saving attachment: ./test.txt\n"))
            .stderr("")
            .success();

        // verify file content
        let file = dir.path().join("test.txt");
        let data = fs::read_to_string(&file).unwrap();
        assert_eq!(&data, &expected);
        // remove file to avoid existence errors on loop
        fs::remove_file(&file).unwrap();
    }
}

#[tokio::test]
async fn save_single_existing_error() {
    let server = start_server().await;
    server
        .respond(200, TEST_PATH.join("attachments/single-plain-text.json"))
        .await;

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();

    cmd("bite attachments")
        .arg("123")
        .args(["-d", dir_path])
        .assert()
        .stdout(predicate::str::diff(format!(
            "Saving attachment: {dir_path}/test.txt\n"
        )))
        .stderr("")
        .success();

    // re-running causes a file existence failure
    cmd("bite attachments")
        .arg("123")
        .args(["-d", dir_path])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(format!(
            "bite: error: file already exists: {dir_path}/test.txt\n"
        )))
        .failure();
}

#[tokio::test]
async fn single_bug_with_no_attachments() {
    let server = start_server().await;

    server
        .respond(
            200,
            TEST_PATH.join("attachments/bug-with-no-attachments.json"),
        )
        .await;

    for subcmd in ["a", "attachments"] {
        for opt in ["-i", "--item-id"] {
            cmd("bite")
                .arg(subcmd)
                .arg("12345")
                .arg(opt)
                .assert()
                .stdout("")
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn multiple_bugs_with_no_attachments() {
    let server = start_server().await;

    server
        .respond(
            200,
            TEST_PATH.join("attachments/bugs-with-no-attachments.json"),
        )
        .await;

    for subcmd in ["a", "attachments"] {
        for opt in ["-i", "--item-id"] {
            cmd("bite")
                .arg(subcmd)
                .args(["12345", "23456", "34567"])
                .arg(opt)
                .assert()
                .stdout("")
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_PATH.join("errors/nonexistent-bug.json"))
        .await;

    for subcmd in ["a", "attachments"] {
        for opt in ["-i", "--item-id"] {
            cmd("bite")
                .arg(subcmd)
                .arg("1")
                .arg(opt)
                .assert()
                .stdout("")
                .stderr("bite: error: bugzilla: Bug #1 does not exist.\n")
                .failure();
        }
    }
}
