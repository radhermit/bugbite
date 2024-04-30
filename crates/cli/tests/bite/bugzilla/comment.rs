use predicates::prelude::*;

use crate::command::cmd;

#[test]
fn missing_ids() {
    cmd("bite bugzilla comment")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}
