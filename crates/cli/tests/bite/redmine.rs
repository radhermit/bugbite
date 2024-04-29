use predicates::str::contains;

use crate::command::cmd;

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite")
            .args([opt, "gentoo"])
            .arg("redmine")
            .assert()
            .stdout("")
            .stderr(contains("redmine not compatible with connection: gentoo"))
            .failure();
    }
}

#[test]
fn no_connection() {
    for action in ["s", "search"] {
        cmd("bite redmine")
            .args([action, "-c", "1d"])
            .assert()
            .stdout("")
            .stderr(contains("no connection specified"))
            .failure();
    }
}
