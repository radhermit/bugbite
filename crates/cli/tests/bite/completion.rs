use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

#[test]
fn no_target() {
    cmd("bite completion")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}

#[test]
fn dir() {
    let dir = tempdir().unwrap();
    for opt in ["-d", "--dir"] {
        cmd("bite completion")
            .arg(opt)
            .arg(dir.path())
            .assert()
            .stdout("")
            .stderr("")
            .success();
        assert!(fs::read_dir(dir.path()).unwrap().next().is_some());
    }
}

#[test]
fn target() {
    // invalid
    cmd("bite completion unknown")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);

    // valid
    cmd("bite completion zsh")
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();
}
