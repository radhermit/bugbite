#![cfg(test)]
use bugbite::test::reset_stdin;
use clap::{CommandFactory, Parser};

use crate::options::Command;

pub(crate) fn subcmd_parse_examples(command: &[&str]) {
    let service = command[0];
    let mut cmd = &mut Command::command();
    for name in command {
        cmd = cmd.find_subcommand_mut(name).unwrap();
    }

    let help = cmd.render_long_help().to_string();
    for line in help.lines() {
        if let Some(example) = line.trim().strip_prefix("> ") {
            for cmd in example.split(" | ").filter_map(|x| x.strip_prefix("bite")) {
                let full_cmd = format!("bite {service} {cmd}");
                let args = shlex::split(full_cmd.trim()).unwrap();
                let result = Command::try_parse_from(args);
                reset_stdin();
                assert!(
                    result.is_ok(),
                    "failed parsing: bite {example}: {}",
                    result.unwrap_err()
                );
            }
        }
    }
}
