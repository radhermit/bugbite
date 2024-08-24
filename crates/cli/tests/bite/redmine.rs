use predicates::str::contains;

use crate::command::cmd;

use super::*;

mod get;

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite redmine")
            .args([opt, "gentoo"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("incompatible connection: gentoo"))
            .failure();
    }
}

#[test]
fn unknown_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite redmine")
            .args([opt, "unknown"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(contains("unknown connection: unknown"))
            .failure();
    }
}
