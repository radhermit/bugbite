#![allow(unused)]

use std::path::PathBuf;
use std::{env, fs, io};

use bugbite::service::ServiceKind;
use clap::CommandFactory;
use strum::IntoEnumIterator;

mod options;
mod service;
mod utils;

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let out_dir = PathBuf::from(args.get(1).map(|x| x.as_str()).unwrap_or("man"));
    fs::create_dir_all(&out_dir).expect("failed creating output directory");
    for kind in ServiceKind::iter() {
        let cmd = match kind {
            ServiceKind::BugzillaRestV1 => service::bugzilla::Command::command(),
            ServiceKind::Github => service::github::Command::command(),
        };
        clap_mangen::generate_to(cmd, &out_dir)?;
    }
    Ok(())
}
