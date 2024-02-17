#![allow(unused)]

use std::path::PathBuf;
use std::{env, fs, io};

use clap::{CommandFactory, ValueEnum};
use clap_complete::Shell;

mod config;
mod macros;
mod options;
mod service;
mod subcmds;
mod utils;

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let out_dir = PathBuf::from(args.get(1).map(|x| x.as_str()).unwrap_or("shell"));
    fs::create_dir_all(&out_dir).expect("failed creating output directory");
    let mut cmd = options::Command::command();
    for &shell in Shell::value_variants() {
        clap_complete::generate_to(shell, &mut cmd, "bite", &out_dir)?;
    }
    Ok(())
}
