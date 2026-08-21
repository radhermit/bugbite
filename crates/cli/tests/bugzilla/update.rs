use bugbite::traits::RequestSend;
use camino_tempfile::tempdir;
use itertools::Itertools;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn from_template() -> anyhow::Result<()> {
    let id = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await?;

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");

    // create template
    cmd!("bite bugzilla update -S new-summary --to {path} --dry-run")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // use template to update bug
    cmd!("bite bugzilla update {id} --from {path}")
        .assert()
        .success();

    let bug = SERVICE.get([id]).send().await?.into_iter().next().unwrap();

    assert_eq!(bug.summary.unwrap(), "new-summary");

    Ok(())
}

#[tokio::test]
async fn multiple_bugs() -> anyhow::Result<()> {
    let id1 = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await?;

    let id2 = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await?;

    cmd!("bite bugzilla update {id1} {id2} -S new-summary")
        .assert()
        .success();

    let (bug1, bug2) = SERVICE
        .get([id1, id2])
        .send()
        .await?
        .into_iter()
        .collect_tuple()
        .unwrap();

    assert_eq!(bug1.summary.unwrap(), "new-summary");
    assert_eq!(bug2.summary.unwrap(), "new-summary");

    Ok(())
}
