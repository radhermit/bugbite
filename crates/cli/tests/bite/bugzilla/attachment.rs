use predicates::prelude::*;

use crate::command::cmd;

mod create;
mod get;
mod update;

#[test]
fn aliases() {
    for subcmd in ["a", "attachment"] {
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
