use predicates::prelude::*;
use tempfile::tempdir;
use wiremock::matchers;

use crate::command::cmd;

use super::*;

macro_rules! default_cmd {
    () => {
        $crate::command::cmd("bite bugzilla create")
            .args(["--component", "TestComponent"])
            .args(["--product", "TestProduct"])
            .args(["--summary", "summary"])
            .args(["--description", "description"])
    };
}

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
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("create/creation.json"))
        .await;

    // no auth
    default_cmd!()
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure()
        .code(1);

    // user and password
    for (opt1, opt2) in [("-u", "-p"), ("--user", "--password")] {
        cmd("bite bugzilla")
            .args([opt1, "user", opt2, "pass"])
            .arg("create")
            .args(["--component", "TestComponent"])
            .args(["--product", "TestProduct"])
            .args(["--summary", "summary"])
            .args(["--description", "description"])
            .assert()
            .success();
    }

    // API key
    for opt in ["-k", "--key"] {
        cmd("bite bugzilla")
            .args([opt, "keydata"])
            .arg("create")
            .args(["--component", "TestComponent"])
            .args(["--product", "TestProduct"])
            .args(["--summary", "summary"])
            .args(["--description", "description"])
            .assert()
            .success();
    }
}

#[tokio::test]
async fn creation() {
    let server = start_server_with_auth().await;

    server
        .respond(200, TEST_DATA.join("create/creation.json"))
        .await;

    // default output
    default_cmd!()
        .assert()
        .stdout(predicate::str::diff("123").trim())
        .stderr("")
        .success();

    // verbose terminal output
    default_cmd!()
        .arg("-v")
        .env("BUGBITE_IS_TERMINAL", "1")
        .assert()
        .stdout(predicate::str::diff("Created bug 123").trim())
        .stderr("")
        .success();
}

#[tokio::test]
async fn from_bug() {
    let server = start_server_with_auth().await;

    server
        .respond_match(
            matchers::path("/rest/bug/12345"),
            200,
            TEST_DATA.join("get/single-bug.json"),
        )
        .await;
    server
        .respond_match(
            matchers::path("/rest/bug"),
            200,
            TEST_DATA.join("create/creation.json"),
        )
        .await;

    // description and summary must be specified
    let err = "Error: missing required fields: description, summary";
    cmd("bite bugzilla create")
        .args(["--from-bug", "12345"])
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(err).trim())
        .failure()
        .code(1);

    // valid
    cmd("bite bugzilla create")
        .args(["--from-bug", "12345"])
        .args(["--description", "description"])
        .args(["--summary", "summary"])
        .assert()
        .stdout(predicate::str::diff("123").trim())
        .stderr("")
        .success();
}

#[tokio::test]
async fn template() {
    let server = start_server_with_auth().await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

    // create template
    default_cmd!()
        .arg("--dry-run")
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // overriding existing template
    for input in ["y\n", "Y\n"] {
        default_cmd!()
            .arg("-n")
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
        .respond(200, TEST_DATA.join("create/creation.json"))
        .await;

    // create bug from template
    cmd("bite bugzilla create")
        .args(["--from", path])
        .assert()
        .stdout(predicate::str::diff("123").trim())
        .stderr("")
        .success();
}
