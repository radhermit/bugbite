use bugbite::traits::RequestSend;
use tempfile::tempdir;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn from_template() {
    let id = SERVICE
        .create()
        .unwrap()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send(&SERVICE)
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
        .get(&[id], false, false, false)
        .unwrap()
        .send(&SERVICE)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(bug.summary.unwrap(), "new-summary");
}
