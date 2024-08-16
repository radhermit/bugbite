use bugbite::traits::RequestSend;
use predicates::prelude::*;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn id() {
    let id = SERVICE
        .create()
        .unwrap()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    cmd!("bite bugzilla search --id {id} --fields id")
        .assert()
        .stdout(predicate::eq(id.to_string()).trim())
        .stderr("")
        .success();
}
