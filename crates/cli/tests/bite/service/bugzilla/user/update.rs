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
async fn name() {
    let _server = start_server().await;

    // authentication required
    cmd("bite bugzilla user update user@domain.com -n test")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure()
        .code(1);

    let server = start_server_with_auth().await;

    // single user
    server
        .respond(200, TEST_DATA.join("user/update/single-name.json"))
        .await;
    for opt in ["-n", "--name"] {
        cmd("bite bugzilla user update user@domain.com")
            .args([opt, "test"])
            .assert()
            .stdout(predicate::str::diff("123").trim())
            .stderr("")
            .success();
    }
}
