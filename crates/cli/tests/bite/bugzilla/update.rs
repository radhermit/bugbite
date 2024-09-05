use predicates::prelude::*;
use tempfile::{tempdir, NamedTempFile};
use wiremock::matchers;

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
        .stdout(predicate::str::diff(indoc::indoc! {"
            === Bug #1 ===
            --- Updated fields ---
            None
        "}))
        .stderr("")
        .success();

    // no field changes for comment only updates
    cmd("bite bugzilla update 123 -v")
        .args(["--comment", "comment"])
        .assert()
        .stdout(predicate::str::diff(indoc::indoc! {"
            === Bug #1 ===
            --- Updated fields ---
            None
            --- Added comment ---
            comment
        "}))
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
    cmd("bite bugzilla update --dry-run")
        .args(["--summary", "new summary"])
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // overriding existing template
    for input in ["y\n", "Y\n"] {
        cmd("bite bugzilla update -n")
            .args(["--summary", "new summary"])
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
        .respond(200, TEST_DATA.join("update/summary.json"))
        .await;

    cmd("bite bugzilla update 123 -v")
        .args(["--from", path])
        .assert()
        .stdout(predicate::str::diff(indoc::indoc! {"
            === Bug #123 ===
            --- Updated fields ---
            summary: old summary -> new summary
        "}))
        .stderr("")
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
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #123 ===
                --- Updated fields ---
                summary: old summary -> new summary
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
#[cfg_attr(target_os = "macos", ignore)] // requires GNU sed which isn't installed by default
async fn reply() {
    let server = start_server_with_auth().await;

    // override interactive editor default
    env::set_var("EDITOR", "sed -i -e '$a\\\n\\\nreply'");

    for opt in ["-R", "--reply"] {
        // invalid
        cmd("bite bugzilla update 1 2")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr(predicate::str::diff("Error: reply must target a single bug").trim())
            .failure()
            .code(1);

        // no comments
        server.reset().await;
        server
            .respond_match(
                matchers::path("/rest/bug/1/comment"),
                200,
                TEST_DATA.join("comment/nonexistent.json"),
            )
            .await;
        cmd("bite bugzilla update 1")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr(predicate::str::diff("Error: reply invalid, bug 1 has no comments").trim())
            .failure()
            .code(1);

        server.reset().await;
        server
            .respond_match(
                matchers::path("/rest/bug/1/comment"),
                200,
                TEST_DATA.join("comment/single-bug.json"),
            )
            .await;
        server
            .respond_match(
                matchers::path("/rest/bug/1"),
                200,
                TEST_DATA.join("update/no-changes.json"),
            )
            .await;

        // invalid comment ID
        cmd("bite bugzilla update 1")
            .args([opt, "7"])
            .assert()
            .stdout("")
            .stderr(predicate::str::diff("Error: reply invalid, nonexistent comment #7").trim())
            .failure()
            .code(1);

        // editor returned failure
        cmd("bite bugzilla update 1")
            .arg(opt)
            .env("EDITOR", "sed -i -e '0d'")
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("Error: failed editing comment"))
            .failure()
            .code(1);

        // no output by default
        cmd("bite bugzilla update 1")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr("")
            .success();

        // last comment default
        cmd("bite bugzilla update 1 -v")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                (In reply to user1 from comment #6)
                > tags

                reply
            "}))
            .stderr("")
            .success();

        // no changes made
        cmd("bite bugzilla update 1 -v")
            .arg(opt)
            .env("EDITOR", "sed -i -e '100d'")
            .write_stdin("Y\n")
            .assert()
            .stderr(predicate::str::diff("No changes made, submit anyway? (y/N):").trim())
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                (In reply to user1 from comment #6)
                > tags
            "}))
            .success();

        // specific comment ID
        cmd("bite bugzilla update 1 -v")
            .args([opt, "4"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                (In reply to user2 from comment #4)
                > comment

                reply
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn comment() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("update/no-changes.json"))
        .await;

    // override interactive editor default
    env::set_var("EDITOR", "tee");

    for opt in ["-c", "--comment"] {
        // no output by default
        cmd("bite bugzilla update 1")
            .args([opt, "comment"])
            .assert()
            .stdout("")
            .stderr("")
            .success();

        // verbose output
        cmd("bite bugzilla update 1 -v")
            .args([opt, "static"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                static
            "}))
            .stderr("")
            .success();

        // comment from stdin
        cmd("bite bugzilla update 1 -v")
            .args([opt, "-"])
            .write_stdin("comment\n")
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                comment
            "}))
            .stderr("")
            .success();

        // option used without argument spawns editor
        cmd("bite bugzilla update 1 -v")
            .arg(opt)
            .write_stdin("interactive\n")
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                interactive
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn comment_from() {
    let server = start_server_with_auth().await;
    server
        .respond(200, TEST_DATA.join("update/no-changes.json"))
        .await;

    for opt in ["-F", "--comment-from"] {
        // missing path
        cmd("bite bugzilla update 1")
            .arg(opt)
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("a value is required"))
            .failure()
            .code(2);

        // nonexistent path
        cmd("bite bugzilla update 1")
            .args([opt, "nonexistent"])
            .assert()
            .stdout("")
            .stderr(predicate::str::contains(
                "failed reading comment file: nonexistent",
            ))
            .failure()
            .code(1);

        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        // empty file
        cmd("bite bugzilla update 1")
            .args([opt, path])
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("empty comment file"))
            .failure()
            .code(1);

        fs::write(path, "comment-from-file").unwrap();

        // verbose output
        cmd("bite bugzilla update 1 -v")
            .args([opt, path])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                === Bug #1 ===
                --- Updated fields ---
                None
                --- Added comment ---
                comment-from-file
            "}))
            .stderr("")
            .success();
    }
}
