use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["s", "search"] {
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

#[test]
fn invalid_ids() {
    cmd("bite bugzilla search")
        .args(["--id", "id"])
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn multiple_stdin() {
    cmd("bite bugzilla search --id - -")
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
async fn fields() {
    let server = start_server().await;

    server.respond(200, TEST_DATA.join("search/ids.json")).await;

    for opt in ["-f", "--fields"] {
        // invalid field
        cmd("bite bugzilla search test")
            .args([opt, "field"])
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("invalid filter field"))
            .failure()
            .code(2);

        // IDS only
        cmd("bite bugzilla search test")
            .args([opt, "id"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                924847
                924852
                924854
                924855
                924856
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn no_matches() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    for opt in ["", "-v", "--verbose"] {
        cmd("bite bugzilla search nonexistent")
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
    cmd("bite bugzilla search --dry-run test")
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // overriding existing template
    for input in ["y\n", "Y\n", "n\n", "N\n", "\n", "yes\ny\n", "no\nn\n"] {
        cmd("bite bugzilla search -n test")
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
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    cmd("bite bugzilla search")
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
        cmd("bite bugzilla search test")
            .arg(opt)
            .env("BROWSER", "true")
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}
