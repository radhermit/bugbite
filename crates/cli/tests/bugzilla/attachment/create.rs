use std::fs;

use bugbite::traits::RequestSend;
use predicates::prelude::*;
use tempfile::NamedTempFile;

use crate::command::cmd;

use super::SERVICE;

#[tokio::test]
async fn single_attachment_to_single_bug() {
    let id = SERVICE
        .create()
        .summary("summary")
        .component("TestComponent")
        .product("TestProduct")
        .description("description")
        .send()
        .await
        .unwrap();

    let file = NamedTempFile::new().unwrap();
    fs::write(&file, "test").unwrap();
    let path = file.path().to_str().unwrap();

    cmd!("bite bugzilla attachment create {id} -v")
        .arg(path)
        .assert()
        .stdout(predicate::str::contains(format!(
            "{path}: attached to bug(s): {id}"
        )))
        .stderr("")
        .success();
}
