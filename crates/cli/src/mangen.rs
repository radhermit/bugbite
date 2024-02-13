#![allow(unused)]

use std::path::PathBuf;
use std::{env, io};

use bugbite::service::ServiceKind;
use clap::CommandFactory;
use strum::IntoEnumIterator;

mod options;
mod service;
mod utils;

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set"));
    for kind in ServiceKind::iter() {
        let cmd = match kind {
            ServiceKind::BugzillaRestV1 => service::bugzilla::Command::command(),
            ServiceKind::Github => service::github::Command::command(),
        };
        clap_mangen::generate_to(cmd, &out_dir)?;
    }
    Ok(())
}
