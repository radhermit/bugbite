use predicates::str::contains;

use crate::command::cmd;

#[test]
fn missing_ids() {
    cmd("bite bugzilla comment")
        .assert()
        .stdout("")
        .stderr(contains("required arguments were not provided"))
        .failure()
        .code(2);
}
