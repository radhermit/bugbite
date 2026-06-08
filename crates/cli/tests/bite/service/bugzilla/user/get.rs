use predicates::prelude::*;

use super::*;

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla user get")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}

#[tokio::test]
async fn nonexistent_user() {
    let server = start_server().await;

    // email
    server
        .respond(200, TEST_DATA.join("user/get/nonexistent-email.json"))
        .await;
    cmd("bite bugzilla user get nonexistent@domain.com")
        .assert()
        .stdout("")
        .stderr(
            predicate::str::diff(
                "Error: bugzilla: There is no user named 'nonexistent@domain.com'.",
            )
            .trim(),
        )
        .failure();

    server.reset().await;

    // id
    server
        .respond(200, TEST_DATA.join("user/get/nonexistent-id.json"))
        .await;
    cmd("bite bugzilla user get 123")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}

#[tokio::test]
async fn single_user() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("user/get/single-unauthenticated.json"))
        .await;

    // email
    cmd("bite bugzilla user get user@domain.com")
        .assert()
        .stdout(predicate::str::diff("A User (user)").trim())
        .stderr("")
        .success();

    // pull email from stdin
    cmd("bite bugzilla user get -")
        .write_stdin("user@domain.com\n")
        .assert()
        .stdout(predicate::str::diff("A User (user)").trim())
        .stderr("")
        .success();
}
