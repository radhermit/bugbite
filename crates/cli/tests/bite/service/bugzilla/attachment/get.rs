use std::{env, fs};

use camino_tempfile::{NamedUtf8TempFile, tempdir};

use super::*;

#[test]
fn aliases() {
    for subcmd in ["g", "get"] {
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
    cmd!("bite bugzilla attachment get")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn invalid_ids() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/get/single-plain-text.json"))
        .await;

    // string IDs only work with -i/--item-ids
    cmd!("bite bugzilla attachment get abc")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: invalid attachment ID: abc").trim())
        .failure()
        .code(1);

    for opt in ["-i", "--item-ids"] {
        cmd!("bite bugzilla attachment get abc {opt}")
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd!("bite bugzilla attachment get 1 {opt}")
            .assert()
            .stdout("")
            .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
            .failure();
    }
}

#[tokio::test]
async fn deleted_attachment() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("attachment/get/deleted.json"))
        .await;

    cmd!("bite bugzilla attachment get 21")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: deleted attachment: 21").trim())
        .failure();
}

#[tokio::test]
async fn list() {
    let server = start_server().await;
    server
        .respond(
            200,
            TEST_DATA.join("attachment/get/single-without-data.json"),
        )
        .await;

    for opt in ["-l", "--list"] {
        // conflicts with -d/--dir and -f/--file
        for x in ["-d", "--dir", "-f", "--file"] {
            cmd!("bite bugzilla attachment get 123 {opt} {x} arg")
                .assert()
                .stdout("")
                .stderr(predicate::str::contains("cannot be used with"))
                .failure()
                .code(2);
        }

        // default output for single attachment
        cmd!("bite bugzilla attachment get 123 {opt}")
            .assert()
            .stdout(predicate::str::diff("123: test.txt").trim())
            .stderr("")
            .success();

        // verbose output for single attachment
        cmd!("bite bugzilla attachment get 123 -v {opt}")
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                123: test.txt
                  (text/plain) 8 B, created by person, 2024-02-19 08:35:02 UTC
            "}))
            .stderr("")
            .success();
    }

    server.reset().await;
    server
        .respond(
            200,
            TEST_DATA.join("attachment/get/multiple-without-data.json"),
        )
        .await;

    for opt in ["-l", "--list"] {
        // default output for multiple attachments
        cmd!("bite bugzilla attachment get 123 124 125 126 {opt}")
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                123: test file 1 (test1)
                124: test file 2 (test2.txt)
            "}))
            .stderr("")
            .success();

        // include obsolete attachments
        for x in ["-o", "--obsolete"] {
            cmd!("bite bugzilla attachment get 123 124 125 126 {opt} {x}")
                .assert()
                .stdout(predicate::str::diff(indoc::indoc! {"
                    123: test file 1 (test1)
                    124: test file 2 (test2.txt)
                    125: patch file (test.patch) (obsolete)
                    126: patch file (test.patch) (deleted)
                "}))
                .stderr("")
                .success();
        }

        // verbose output for multiple attachments
        cmd!("bite bugzilla attachment get 123 124 125 126 -v {opt}")
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                123: test file 1 (test1)
                  (text/plain) 8 B, created by person, 2024-02-19 08:35:02 UTC
                124: test file 2 (test2.txt)
                  (text/plain) 8 B, created by person, 2024-02-19 08:35:02 UTC
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn output_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/get/single-plain-text.json"))
        .await;
    let expected =
        fs::read_to_string(TEST_OUTPUT.join("attachment/get/single-plain-text")).unwrap();

    for opt in ["-f", "--file"] {
        // stdout target
        cmd!("bite bugzilla attachment get 123")
            .args([opt, "-"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();

        // file target
        let file = NamedUtf8TempFile::new().unwrap();
        let path = file.path().as_str();
        cmd!("bite bugzilla attachment get 123")
            .args([opt, path])
            .assert()
            .stdout("")
            .stderr("")
            .success();
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, expected);
    }
}

#[tokio::test]
async fn save_single_with_plain_text() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("attachment/get/single-plain-text.json"))
        .await;
    let expected =
        fs::read_to_string(TEST_OUTPUT.join("attachment/get/single-plain-text")).unwrap();

    let dir = tempdir().unwrap();
    // save files to the current working directory
    env::set_current_dir(dir.path()).unwrap();

    cmd!("bite bugzilla attachment get 123")
        .assert()
        .stdout(predicate::str::diff("Saving attachment: ./test.txt").trim())
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
        .respond(200, TEST_DATA.join("attachment/get/single-plain-text.json"))
        .await;

    let dir = tempdir().unwrap();
    let prefix = dir.path().as_str();

    cmd!("bite bugzilla attachment get 123 -d {prefix}")
        .assert()
        .stdout(predicate::str::diff(format!("Saving attachment: {prefix}/test.txt")).trim())
        .stderr("")
        .success();

    // re-running causes a file existence failure
    cmd!("bite bugzilla attachment get 123 -d {prefix}")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(format!("Error: file exists: {prefix}/test.txt")).trim())
        .failure();

    // but works when forcing
    for opt in ["-F", "--force"] {
        cmd!("bite bugzilla attachment get 123 {opt} -d {prefix}")
            .assert()
            .stdout(predicate::str::diff(format!("Saving attachment: {prefix}/test.txt")).trim())
            .stderr("")
            .success();
    }

    // or when confirming overwrite
    cmd!("bite bugzilla attachment get 123 -d {prefix}")
        .write_stdin("y\n")
        .assert()
        .stdout(predicate::str::diff(format!("Saving attachment: {prefix}/test.txt")).trim())
        .stderr(
            predicate::str::diff(format!("{prefix}/test.txt: file exists, overwrite? (y/N):"))
                .trim(),
        )
        .success();
}

#[tokio::test]
async fn single_bug_with_no_attachments() {
    let server = start_server().await;

    server
        .respond(
            200,
            TEST_DATA.join("attachment/get/bug-with-no-attachments.json"),
        )
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd!("bite bugzilla attachment get 12345 {opt}")
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
            TEST_DATA.join("attachment/get/bugs-with-no-attachments.json"),
        )
        .await;

    for opt in ["-i", "--item-ids"] {
        cmd!("bite bugzilla attachment get 12345 23456 34567 {opt}")
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
        .respond(
            200,
            TEST_DATA.join("attachment/get/bugs-with-attachments.json"),
        )
        .await;
    let expected =
        fs::read_to_string(TEST_OUTPUT.join("attachment/get/single-plain-text")).unwrap();

    let dir = tempdir().unwrap();
    // save files to the current working directory
    env::set_current_dir(dir.path()).unwrap();

    let ids = ["12345", "23456", "34567"];
    cmd!("bite bugzilla attachment get -i")
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
