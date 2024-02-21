use predicates::prelude::*;
use predicates::str::contains;

use crate::command::cmd;

use super::set_fake_env;

#[test]
fn missing_ids() {
    set_fake_env();
    cmd("bite comments")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn invalid_ids() {
    set_fake_env();
    cmd("bite comments")
        .arg("id")
        .assert()
        .stdout("")
        .stderr(contains("error: invalid value 'id' for '<IDS>...': "))
        .failure()
        .code(2);
}
