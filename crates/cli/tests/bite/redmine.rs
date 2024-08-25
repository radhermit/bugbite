use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;

use crate::command::cmd;

use super::*;

mod get;
mod search;

static TEST_DATA: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("bugbite/redmine"));

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite redmine")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn incompatible_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite redmine")
            .args([opt, "gentoo"])
            .args(["search", "test"])
            .assert()
            .stdout("")
            .stderr(predicate::str::contains("incompatible connection: gentoo"))
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
            .stderr(predicate::str::contains("unknown connection: unknown"))
            .failure();
    }
}
