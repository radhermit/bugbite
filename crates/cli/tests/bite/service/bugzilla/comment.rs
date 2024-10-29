use std::fs;

use predicates::prelude::*;

use super::*;

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla comment")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
        .failure();
}

#[tokio::test]
async fn nonexistent() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/nonexistent.json"))
        .await;

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn description() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/description.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/description")).unwrap();

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn single_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/single-bug.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/single-bug")).unwrap();

    cmd("bite bugzilla comment 1")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull id from stdin
    cmd("bite bugzilla comment -")
        .write_stdin("1\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn multiple_bugs() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/multiple-bugs.json"))
        .await;
    let expected = fs::read_to_string(TEST_OUTPUT.join("comment/multiple-bugs")).unwrap();

    cmd("bite bugzilla comment 1 2")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();

    // pull ids from stdin
    cmd("bite bugzilla comment -")
        .write_stdin("1\n2\n")
        .assert()
        .stdout(predicate::str::diff(expected.clone()))
        .stderr("")
        .success();
}

#[tokio::test]
async fn creator() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/single-bug.json"))
        .await;

    for opt in ["-R", "--creator"] {
        cmd("bite bugzilla comment 1")
            .args([opt, "user1"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                Bug: 1 ===================================================================================
                Description by user1@bugbite.test, 2024-03-13 14:02:53 UTC
                ------------------------------------------------------------------------------------------
                test

                Comment #1 by user1@bugbite.test, 2024-03-13 14:04:31 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 123
                test.patch

                Comment #2 (private) by user1@bugbite.test, 2024-03-13 14:05:02 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 234
                test data

                Comment #6 (spam, test) by user1@bugbite.test, 2024-03-13 14:46:57 UTC
                ------------------------------------------------------------------------------------------
                tags
            "}))
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn attachment() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/single-bug.json"))
        .await;

    for opt in ["-a", "--attachment"] {
        // comments with attachments
        cmd("bite bugzilla comment 1")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                Bug: 1 ===================================================================================
                Comment #1 by user1@bugbite.test, 2024-03-13 14:04:31 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 123
                test.patch

                Comment #2 (private) by user1@bugbite.test, 2024-03-13 14:05:02 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 234
                test data

                Comment #3 by user2@bugbite.test, 2024-03-13 14:11:47 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 345
                build log
            "}))
            .stderr("")
            .success();

        // comments without attachments
        cmd("bite bugzilla comment 1")
            .args([opt, "false"])
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                Bug: 1 ===================================================================================
                Description by user1@bugbite.test, 2024-03-13 14:02:53 UTC
                ------------------------------------------------------------------------------------------
                test

                Comment #4 by user2@bugbite.test, 2024-03-13 14:45:00 UTC
                ------------------------------------------------------------------------------------------
                comment

                Comment #5 (private) by user2@bugbite.test, 2024-03-13 14:46:29 UTC
                ------------------------------------------------------------------------------------------
                private

                Comment #6 (spam, test) by user1@bugbite.test, 2024-03-13 14:46:57 UTC
                ------------------------------------------------------------------------------------------
                tags
            "}))
            .stderr("")
            .success();

        // comments with attachments by a specific user
        cmd("bite bugzilla comment 1 --creator user2")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(indoc::indoc! {"
                Bug: 1 ===================================================================================
                Comment #3 by user2@bugbite.test, 2024-03-13 14:11:47 UTC
                ------------------------------------------------------------------------------------------
                Created attachment 345
                build log
            "}))
            .stderr("")
            .success();
    }
}
