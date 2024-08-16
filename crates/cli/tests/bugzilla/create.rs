use bugbite::traits::RequestSend;
use tempfile::tempdir;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn from_bug() {
    let id = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    cmd!("bite bugzilla create --from-bug {id} -S summary -D description")
        .assert()
        .success();
}

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

    // create template from bug
    cmd!("bite bugzilla create --from-bug {id} --to {path} --dry-run")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // use template to create bug
    cmd!("bite bugzilla create --from {path} -S summary -D description")
        .assert()
        .success();
}
