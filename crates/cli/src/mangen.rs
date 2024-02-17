#![allow(unused)]

use std::path::PathBuf;
use std::{env, fs, io};

use clap::CommandFactory;

mod config;
mod macros;
mod options;
mod service;
mod subcmds;
mod utils;

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let out_dir = PathBuf::from(args.get(1).map(|x| x.as_str()).unwrap_or("man"));
    fs::create_dir_all(&out_dir).expect("failed creating output directory");
    let cmd = options::Command::command();
    clap_mangen::generate_to(cmd, &out_dir)?;
    Ok(())
}
