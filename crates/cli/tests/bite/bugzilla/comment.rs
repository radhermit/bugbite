use predicates::prelude::*;

use crate::command::cmd;

#[test]
fn missing_ids() {
    cmd("bite bugzilla comment")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);
}
