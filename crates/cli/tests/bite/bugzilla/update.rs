use predicates::prelude::*;
use tempfile::tempdir;

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

#[tokio::test]
async fn no_changes() {
    let server = start_server_with_auth().await;

    server
        .respond(200, TEST_DATA.join("update/no-changes.json"))
        .await;

    // no field changes if new value is the same as the original value
    cmd("bite bugzilla update 123 -v")
        .args(["--summary", "new summary"])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(indoc::indoc! {"
            === Bug #123 ===
            --- Updated fields ---
            None
        "}))
        .success();

    // no field changes for comment only updates
    cmd("bite bugzilla update 123 -v")
        .args(["--comment", "comment"])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(indoc::indoc! {"
            === Bug #123 ===
            --- Updated fields ---
            None
            --- Added comment ---
            comment
        "}))
        .success();
}

#[tokio::test]
async fn template() {
    let server = start_server_with_auth().await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

    // create template
    cmd("bite bugzilla update --dry-run")
        .args(["--summary", "new summary"])
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // overriding existing template
    for input in ["y\n", "Y\n", "n\n", "N\n", "\n"] {
        cmd("bite bugzilla update --dry-run")
            .args(["--summary", "new summary"])
            .args(["--to", path])
            .write_stdin(input)
            .assert()
            .stdout(predicate::str::contains(format!(
                "template exists: {path}, overwrite?"
            )))
            .stderr("")
            .success();
    }

    server
        .respond(200, TEST_DATA.join("update/summary.json"))
        .await;

    cmd("bite bugzilla update 123 -v")
        .args(["--from", path])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(indoc::indoc! {"
            === Bug #123 ===
            --- Updated fields ---
            summary: old summary -> new summary
        "}))
        .success();
}

#[tokio::test]
async fn summary() {
    let server = start_server_with_auth().await;

    server
        .respond(200, TEST_DATA.join("update/summary.json"))
        .await;

    for opt in ["-S", "--summary"] {
        cmd("bite bugzilla update 123")
            .args([opt, "new summary"])
            .assert()
            .stdout("")
            .stderr("")
            .success();

        // verify output when running verbosely
        cmd("bite bugzilla update 123 -v")
            .args([opt, "new summary"])
            .assert()
            .stdout("")
            .stderr(predicate::str::diff(indoc::indoc! {"
                === Bug #123 ===
                --- Updated fields ---
                summary: old summary -> new summary
            "}))
            .success();
    }
}

#[tokio::test]
async fn reply() {
    let server = start_server_with_auth().await;

    server
        .respond(200, TEST_DATA.join("update/no-changes.json"))
        .await;

    for opt in ["-R", "--reply"] {
        cmd("bite bugzilla update 123 124")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr(predicate::str::diff("Error: reply invalid, targeting multiple bugs").trim())
            .failure()
            .code(1);
    }
}
