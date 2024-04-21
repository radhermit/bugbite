#![cfg(test)]
use std::{env, fs};

use bugbite::test::{build_path, reset_stdin};

use crate::options::Command;

/// Parse examples from documentation.
pub(crate) fn subcmd_parse_doc(doc: &str) {
    // wipe bugbite-related environment variables
    for (key, _value) in env::vars() {
        if key.starts_with("BUGBITE_") {
            env::remove_var(key);
        }
    }

    let file = build_path!(env!("CARGO_MANIFEST_DIR"), "doc", format!("{doc}.adoc"));
    let doc = fs::read_to_string(file).unwrap();
    for line in doc.lines().filter(|x| x.starts_with(' ')) {
        for cmd in line.trim().split(" | ").filter(|x| x.starts_with("bite ")) {
            let args = shlex::split(cmd).unwrap();
            // TODO: fix parse_args() to return errors for tests instead of exiting
            let result = Command::parse_args(args);
            reset_stdin();
            assert!(
                result.is_ok(),
                "failed parsing: {cmd}\n{}",
                result.unwrap_err()
            );
        }
    }
}
