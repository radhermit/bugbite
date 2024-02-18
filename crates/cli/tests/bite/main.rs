use predicates::prelude::*;

mod bugzilla;
mod command;
mod macros;

use command::cmd;

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn version() {
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}
