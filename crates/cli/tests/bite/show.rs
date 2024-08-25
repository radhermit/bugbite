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

#[test]
fn connections() {
    cmd("bite show connections")
        .assert()
        .stdout(predicate::str::contains("gentoo"))
        .stderr("")
        .success();
}
