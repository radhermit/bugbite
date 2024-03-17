use std::str;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

#[test]
fn bug_id_output() {
    cmd!("bite create -S summary -C TestComponent -p TestProduct -D description")
        .assert()
        .stdout(predicate::str::is_match(r"^\d+$").unwrap().trim())
        .stderr("")
        .success();
}

#[test]
fn from_bug() {
    let output = cmd!("bite create -S summary -C TestComponent -p TestProduct -D description")
        .output()
        .unwrap();
    let id = str::from_utf8(&output.stdout).unwrap().trim();

    cmd!("bite create -S summary -D description --from-bug {id}")
        .assert()
        .stdout(predicate::str::is_match(r"^\d+$").unwrap().trim())
        .stderr("")
        .success();
}

#[test]
fn from_template() {
    let output = cmd!("bite create -S summary -C TestComponent -p TestProduct -D description")
        .output()
        .unwrap();
    let id = str::from_utf8(&output.stdout).unwrap().trim();

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
