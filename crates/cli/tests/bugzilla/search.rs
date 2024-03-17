use std::str;

use predicates::prelude::*;

use crate::command::cmd;

#[test]
fn id() {
    let output = cmd!("bite create -S summary -C TestComponent -p TestProduct -D description")
        .output()
        .unwrap();
    let id = str::from_utf8(&output.stdout).unwrap().trim();

    cmd!("bite search --id {id} --fields id")
        .assert()
        .stdout(predicate::eq(id).trim())
        .stderr("")
        .success();
}
