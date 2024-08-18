use predicates::prelude::*;

use crate::command::cmd;

#[test]
fn required_args() {
    // missing IDs
    cmd("bite bugzilla history")
        .assert()
        .stdout("")
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ))
        .failure()
        .code(2);
}
