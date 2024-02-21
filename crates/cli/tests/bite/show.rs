use predicates::prelude::*;

use crate::command::cmd;

#[test]
fn services() {
    cmd("bite show services")
        .assert()
        .stdout(predicate::str::starts_with("Service: "))
        .stderr("")
        .success();
}
