use predicates::prelude::*;

use crate::command::cmd;

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
