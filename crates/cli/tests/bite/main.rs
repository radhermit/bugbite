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
    // wipe bugbite-related environment variables
    for (key, _value) in env::vars() {
        if key.starts_with("BUGBITE_") {
            env::remove_var(key);
        }
    }
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
    let version = env!("CARGO_PKG_VERSION");
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(format!("bite {version}")).trim())
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

#[test]
fn service_arg_conflicts() {
    // -c/--connection can't be used with -s/--service
    cmd("bite -c gentoo -s redmine")
        .assert()
        .stdout("")
        .stderr(contains("--connection"))
        .code(2)
        .failure();

    // -c/--connection can't be used with -b/--base
    cmd("bite -c gentoo -b https://service/url")
        .assert()
        .stdout("")
        .stderr(contains("--connection"))
        .code(2)
        .failure();
}
