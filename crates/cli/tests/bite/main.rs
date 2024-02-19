use bugbite::test::build_path;
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use predicates::prelude::*;

mod bugzilla;
mod command;

use command::cmd;

pub(crate) static TESTDATA_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn version() {
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}
