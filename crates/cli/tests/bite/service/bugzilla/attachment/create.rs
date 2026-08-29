use std::{env, fs};

use camino_tempfile::{NamedUtf8TempFile, tempdir};

use super::*;

#[test]
fn aliases() {
    for subcmd in ["c", "create"] {
        for opt in ["-h", "--help"] {
            cmd!("bite bugzilla attachment {subcmd} {opt}")
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
    cmd!("bite bugzilla attachment create")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);

    // missing files
    cmd!("bite bugzilla attachment create 1")
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
    let file = NamedUtf8TempFile::new().unwrap();
    fs::write(&file, "test").unwrap();
    let path = file.path();

    cmd!("bite bugzilla attachment create 1 {path}")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure()
        .code(1);
}

#[tokio::test]
async fn empty_file() {
    let _server = start_server().await;
    let file = NamedUtf8TempFile::new().unwrap();
    let path = file.path();

    cmd!("bite bugzilla attachment create 1 {path}")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff(format!("Error: empty attachment: {path}")).trim())
        .failure()
        .code(1);
}

#[tokio::test]
async fn nonexistent_file() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("attachment/create/single.json"))
        .await;

    cmd!("bite bugzilla attachment create 1 /path/to/nonexistent/file")
        .assert()
        .stdout("")
        .stderr(
            predicate::str::diff("Error: failed reading attachment: /path/to/nonexistent/file: No such file or directory (os error 2)")
                .trim(),
        )
        .failure()
        .code(1);
}

#[tokio::test]
async fn single_bug() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("attachment/create/single.json"))
        .await;

    let file = NamedUtf8TempFile::new().unwrap();
    fs::write(&file, "test").unwrap();
    let path = file.path();

    cmd!("bite bugzilla attachment create 1 {path}")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // verify output when running verbosely
    cmd!("bite bugzilla attachment create 1 -v {path}")
        .assert()
        .stdout(predicate::str::diff(indoc::formatdoc! {"
            {path}: attached to bug(s): 1 (attachment ID(s) 123)
        "}))
        .stderr("")
        .success();

    // IDs from standard input
    cmd!("bite bugzilla attachment create - {path}")
        .write_stdin("1\n")
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

    let file = NamedUtf8TempFile::new().unwrap();
    fs::write(&file, "test").unwrap();
    let path = file.path();

    // invalid command -- ID args must be in a single comma-separated string
    cmd!("bite bugzilla attachment create 1 2 {path}")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "Error: failed reading attachment: 2",
        ))
        .failure()
        .code(1);

    cmd!("bite bugzilla attachment create 1,2 {path}")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // verify output when running verbosely
    cmd!("bite bugzilla attachment create 1,2 -v {path}")
        .assert()
        .stdout(predicate::str::diff(indoc::formatdoc! {"
            {path}: attached to bug(s): 1, 2 (attachment ID(s) 123, 124)
        "}))
        .stderr("")
        .success();

    // IDs from standard input
    cmd!("bite bugzilla attachment create - {path}")
        .write_stdin("1\n2\n")
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
    cmd!("bite bugzilla attachment create 1 src")
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
    cmd!("bite bugzilla attachment create 1 src --patch")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: patch type invalid for directory targets").trim())
        .failure()
        .code(1);

    // empty directory target
    cmd!("bite bugzilla attachment create 1 src")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: empty directory target: src").trim())
        .failure()
        .code(1);

    // create files
    fs::write("src/test1", "test1").unwrap();
    fs::write("src/test2", "test2").unwrap();

    // valid
    cmd!("bite bugzilla attachment create 1 src")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
