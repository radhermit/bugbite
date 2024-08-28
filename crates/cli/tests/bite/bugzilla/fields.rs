use predicates::prelude::*;

use crate::command::cmd;

use super::*;

#[tokio::test]
async fn fields() {
    let server = start_server().await;
    server
        .respond(200, TEST_DATA.join("fields/gentoo.json"))
        .await;

    cmd("bite bugzilla fields")
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();
}
