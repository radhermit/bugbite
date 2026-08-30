use super::*;

#[test]
fn aliases() {
    for subcmd in ["u", "update"] {
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
    cmd!("bite bugzilla attachment update")
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

    cmd!("bite bugzilla attachment update 1 -p")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}

#[tokio::test]
async fn nonexistent() {
    let server = start_server_with_auth().await;

    server
        .respond(400, TEST_DATA.join("attachment/update/nonexistent.json"))
        .await;

    cmd!("bite bugzilla attachment update -d test 0")
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: bugzilla: The attachment id 0 is invalid.").trim())
        .failure();
}

#[tokio::test]
async fn no_change() {
    let server = start_server_with_auth().await;

    server
        .respond(200, TEST_DATA.join("attachment/update/no-change.json"))
        .await;

    cmd!("bite bugzilla attachment update -d test 1")
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
