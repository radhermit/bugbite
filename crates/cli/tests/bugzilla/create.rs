use bugbite::traits::RequestSend;
use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

use super::SERVICE;

#[test]
fn bug_id_output() {
    cmd!("bite create -S summary -C TestComponent -p TestProduct -D description")
        .assert()
        .stdout(predicate::str::is_match(r"^\d+$").unwrap().trim())
        .stderr("")
        .success();
}

#[tokio::test]
async fn from_bug() {
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

    cmd!("bite create -S summary -D description --from-bug {id}")
        .assert()
        .stdout(predicate::str::is_match(r"^\d+$").unwrap().trim())
        .stderr("")
        .success();
}

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

    // create template from bug
    cmd!("bite create --from-bug {id} --to {path} --dry-run")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // use template to create bug
    cmd!("bite create --from {path} -S summary -D description")
        .assert()
        .stdout(predicate::str::is_match(r"^\d+$").unwrap().trim())
        .stderr("")
        .success();
}
