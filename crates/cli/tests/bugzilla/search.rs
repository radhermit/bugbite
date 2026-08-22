use predicates::prelude::*;

use crate::command::cmd;

use super::get_existing_bug;

#[tokio::test]
async fn id() -> anyhow::Result<()> {
    let id = get_existing_bug().await?;

    cmd!("bite bugzilla search --id {id} --fields id")
        .assert()
        .stdout(predicate::eq(id.to_string()).trim())
        .stderr("")
        .success();

    Ok(())
}
