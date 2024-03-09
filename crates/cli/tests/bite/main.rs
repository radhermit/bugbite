use std::env;

use bugbite::test::build_path;
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;
use predicates::str::contains;

use command::cmd;

mod bugzilla;
mod command;
mod show;

pub(crate) static TEST_DATA_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

/// Initialization for all test executables.
#[ctor::ctor]
fn initialize() {
    // wipe environment variables that affect connections
    env::remove_var("BUGBITE_CONNECTION");
    env::remove_var("BUGBITE_BASE");
    env::remove_var("BUGBITE_SERVICE");
}

// verify help support isn't mangled by service subcommand injection
#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::starts_with("bite"))
            .stderr("")
            .success();
    }
}

// verify version support isn't mangled by service subcommand injection
#[test]
fn version() {
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::starts_with("bite"))
            .stderr("")
            .success();
    }
}

#[test]
fn unknown_connection() {
    for opt in ["-c", "--connection"] {
        cmd("bite")
            .args([opt, "unknown", "--help"])
            .assert()
            .stdout("")
            .stderr(contains("unknown connection: unknown"))
            .failure();
    }
}
