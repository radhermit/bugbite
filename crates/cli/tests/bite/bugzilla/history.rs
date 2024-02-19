use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

#[test]
fn missing_ids() {
    cmd("bite history")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn invalid_ids() {
    cmd("bite history")
        .arg("id")
        .assert()
        .stdout("")
        .stderr(contains("error: invalid value 'id' for '<IDS>...': "))
        .failure()
        .code(2);
}
