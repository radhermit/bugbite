use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["s", "search"] {
        for opt in ["-h", "--help"] {
            cmd("bite redmine")
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
fn invalid_ids() {
    cmd("bite redmine search")
        .args(["--id", "id"])
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn multiple_stdin() {
    cmd("bite redmine search --id - -")
        .write_stdin("12345\n")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "stdin argument used more than once",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn no_matches() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    for opt in ["", "-v", "--verbose"] {
        cmd("bite redmine search nonexistent")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn template() {
    let server = start_server().await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

    // create template
    cmd("bite redmine search --dry-run test")
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // overriding existing template
    for input in ["y\n", "Y\n"] {
        cmd("bite redmine search -n test")
            .args(["--to", path])
            .write_stdin(input)
            .assert()
            .stdout("")
            .stderr(
                predicate::str::diff(format!("template exists: {path}, overwrite? (y/N):")).trim(),
            )
            .success();
    }

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    cmd("bite redmine search")
        .args(["--from", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn browser() {
    let _server = start_server().await;

    for opt in ["-b", "--browser"] {
        cmd("bite redmine search test")
            .arg(opt)
            .env("BROWSER", "true")
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}
