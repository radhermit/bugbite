#![allow(unused)]

use std::path::PathBuf;
use std::{env, fs, io};

use clap::{CommandFactory, ValueEnum};
use clap_complete::Shell;

mod config;
mod options;
mod service;
mod subcmds;
mod utils;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = env::args().collect();
    let mut cmd = options::Command::command();

    // generate shell completions
    fs::create_dir_all("shell").expect("failed creating output directory");
    for &shell in Shell::value_variants() {
        clap_complete::generate_to(shell, &mut cmd, "bite", "shell")?;
    }

    // generate man pages
    fs::create_dir_all("man").expect("failed creating output directory");
    clap_mangen::generate_to(cmd, "man")?;

    Ok(())
}
