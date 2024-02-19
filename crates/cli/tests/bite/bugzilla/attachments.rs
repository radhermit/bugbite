use std::fs;

use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use super::start_server;

static TEST_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| crate::TESTDATA_PATH.join("bugzilla/attachments"));

#[tokio::test]
async fn list_single_via_bug_id_without_data() {
    let server = start_server().await;

    server
        .respond(200, TEST_PATH.join("single-without-data.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("single-without-data.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opts in [vec!["-li"], vec!["-l", "-i"], vec!["--list", "--item-id"]] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn view_single_via_bug_id_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_PATH.join("single-plain-text.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("single-plain-text.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opts in [vec!["-Vi"], vec!["-V", "-i"], vec!["--view", "--item-id"]] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn save_single_via_bug_id_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_PATH.join("single-plain-text.json"))
        .await;
    let expected = fs::read_to_string(TEST_PATH.join("single-plain-text.expected")).unwrap();

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();
    for subcmd in ["a", "attachments"] {
        for opts in [
            vec!["-d", dir_path, "-i"],
            vec!["--dir", dir_path, "--item-id"],
        ] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(format!(
                    "Saving attachment: {dir_path}/test.txt\n"
                )))
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
}
