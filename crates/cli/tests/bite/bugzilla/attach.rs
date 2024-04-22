use predicates::prelude::*;
use tempfile::NamedTempFile;

use crate::command::cmd;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["at", "attach"] {
        for opt in ["-h", "--help"] {
            cmd("bite bugzilla")
                .arg(subcmd)
                .arg(opt)
                .assert()
                .stdout(predicate::str::is_empty().not())
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn auth_required() {
    let _server = start_server().await;
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    cmd("bite attach 1")
        .arg(path)
        .assert()
        .stdout("")
        .stderr(predicate::str::diff("Error: authentication required").trim())
        .failure();
}
