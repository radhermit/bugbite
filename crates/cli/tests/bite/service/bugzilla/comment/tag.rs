use super::*;

#[tokio::test]
async fn required_args() {
    let _server = start_server().await;

    // missing IDs
    cmd("bite bugzilla comment tag")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);

    // missing --tags or --untag
    cmd("bite bugzilla comment tag 1")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains("no tags specified"))
        .failure()
        .code(1);
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(404, TEST_DATA.join("errors/nonexistent-bug.json"))
        .await;

    cmd("bite bugzilla comment tag 1 -u")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: Bug #1 does not exist.").trim())
        .failure();
}

#[tokio::test]
async fn nonexistent_comments() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("comment/get/nonexistent.json"))
        .await;

    cmd("bite bugzilla comment tag 1 -u")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
