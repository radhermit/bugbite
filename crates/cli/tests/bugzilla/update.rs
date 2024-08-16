use bugbite::traits::RequestSend;
use itertools::Itertools;
use tempfile::tempdir;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn from_template() {
    let id = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.to_str().unwrap();

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

    let bug = SERVICE
        .get([id])
        .send()
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(bug.summary.unwrap(), "new-summary");
}

#[tokio::test]
async fn multiple_bugs() {
    let id1 = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    let id2 = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    cmd!("bite bugzilla update {id1} {id2} -S new-summary")
        .assert()
        .success();

    let (bug1, bug2) = SERVICE
        .get([id1, id2])
        .send()
        .await
        .unwrap()
        .into_iter()
        .collect_tuple()
        .unwrap();

    assert_eq!(bug1.summary.unwrap(), "new-summary");
    assert_eq!(bug2.summary.unwrap(), "new-summary");
}
