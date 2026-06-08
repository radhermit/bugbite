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
async fn single_user() {
    let _server = start_server().await;

    // authentication required
    cmd("bite bugzilla user create test")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure()
        .code(1);

    let server = start_server_with_auth().await;

    // invalid user
    server
        .respond(400, TEST_DATA.join("user/create/invalid-user.json"))
        .await;
    cmd("bite bugzilla user create test")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("e-mail address"))
        .failure()
        .code(1);

    server.reset().await;

    // valid user
    server
        .respond(200, TEST_DATA.join("user/create/single.json"))
        .await;
    cmd("bite bugzilla user create user@domain.com")
        .assert()
        .stdout(predicate::str::diff("2").trim())
        .stderr("")
        .success();
}
