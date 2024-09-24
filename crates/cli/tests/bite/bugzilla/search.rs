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

    // output template to stdout
    cmd("bite bugzilla search -c 1d -n")
        .args(["--to", "-"])
        .assert()
        .stdout(predicate::str::diff("created = \"1d\"").trim())
        .stderr("")
        .success();

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

    // save template to a specific path
    cmd("bite bugzilla search -c 1d -n")
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // load template
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

#[tokio::test]
async fn custom_fields() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // missing args
    cmd("bite bugzilla search --cf")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("value is required"))
        .failure()
        .code(2);

    // existing
    cmd("bite bugzilla search --cf field")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // single
    cmd("bite bugzilla search --cf field=value")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // multiple
    cmd("bite bugzilla search --cf field1=value --cf field1='!= value'")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn changed() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // missing field
    cmd("bite bugzilla search --changed")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("value is required"))
        .failure()
        .code(2);

    // single
    cmd("bite bugzilla search --changed alias")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // multiple
    cmd("bite bugzilla search --changed blocks,depends")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // invalid time
    cmd("bite bugzilla search --changed blocks=2")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("invalid range or value: 2"))
        .failure()
        .code(2);

    // time range
    cmd("bite bugzilla search --changed blocks=1d")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // time static
    cmd("bite bugzilla search --changed blocks,depends=2020-02-02")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn changed_by() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // missing args
    cmd("bite bugzilla search --changed-by")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("value is required"))
        .failure()
        .code(2);

    // missing value
    cmd("bite bugzilla search --changed-by alias")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("missing users"))
        .failure()
        .code(2);

    // single
    cmd("bite bugzilla search --changed-by alias=user")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // multiple
    cmd("bite bugzilla search --changed-by blocks,depends=user1,user2")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn changed_from() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // missing args
    cmd("bite bugzilla search --changed-from")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("value is required"))
        .failure()
        .code(2);

    // missing value
    cmd("bite bugzilla search --changed-from alias")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("missing value"))
        .failure()
        .code(2);

    // single
    cmd("bite bugzilla search --changed-from alias=user")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn changed_to() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // missing args
    cmd("bite bugzilla search --changed-to")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("value is required"))
        .failure()
        .code(2);

    // missing value
    cmd("bite bugzilla search --changed-to alias")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("missing value"))
        .failure()
        .code(2);

    // single
    cmd("bite bugzilla search --changed-to alias=user")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn id() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // multiple options
    cmd("bite bugzilla search --id 1 --id 2")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // comma separated args
    for args in ["1,2,3", ">50,<100"] {
        cmd("bite bugzilla search --id")
            .arg(args)
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }

    // comma separated stdin args
    cmd("bite bugzilla search --id -")
        .write_stdin("1,2,3\n")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // newline separated stdin args
    cmd("bite bugzilla search --id -")
        .write_stdin("1\n2\n3\n")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
