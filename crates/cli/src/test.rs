#![cfg(test)]
use std::{env, fs};

use bugbite::test::{build_path, reset_stdin};
use clap::{CommandFactory, Parser};
use itertools::Itertools;

use crate::options::Command;

/// Parse examples from clap long help.
pub(crate) fn subcmd_parse_examples(command: &[&str]) {
    let service = command[0];
    let mut cmd = &mut Command::command();
    for name in command {
        cmd = cmd.find_subcommand_mut(name).unwrap();
    }

    let help = cmd.render_long_help().to_string();
    for line in help.lines() {
        if let Some(example) = line.trim().strip_prefix("> ") {
            for cmd in example.split(" | ").filter_map(|x| x.strip_prefix("bite ")) {
                let full_cmd = format!("bite {service} {cmd}");
                let args = shlex::split(&full_cmd).unwrap();
                let result = Command::try_parse_from(args);
                reset_stdin();
                assert!(
                    result.is_ok(),
                    "failed parsing: bite {cmd}\n{}",
                    result.unwrap_err()
                );
            }
        }
    }
}

/// Parse examples from documentation.
pub(crate) fn subcmd_parse_doc(subcmds: &[&str]) {
    // wipe bugbite-related environment variables
    for (key, _value) in env::vars() {
        if key.starts_with("BUGBITE_") {
            env::remove_var(key);
        }
    }

    let file_name = format!("bite-{}.adoc", subcmds.iter().join("-"));
    let file = build_path!(env!("CARGO_MANIFEST_DIR"), "doc", &file_name);
    let doc = fs::read_to_string(file).unwrap();
    for line in doc.lines() {
        if let Some(example) = line.trim().strip_prefix("$ ") {
            for cmd in example.split(" | ").filter(|x| x.starts_with("bite ")) {
                let args = shlex::split(cmd).unwrap();
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
}
