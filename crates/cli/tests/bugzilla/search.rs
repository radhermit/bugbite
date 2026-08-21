use bugbite::traits::RequestSend;
use predicates::prelude::*;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn id() -> anyhow::Result<()> {
    let id = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await?;

    cmd!("bite bugzilla search --id {id} --fields id")
        .assert()
        .stdout(predicate::eq(id.to_string()).trim())
        .stderr("")
        .success();

    Ok(())
}
