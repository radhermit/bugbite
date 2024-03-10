use std::io::{stderr, Write};
use std::process::ExitCode;

use clap::Parser;

mod config;
mod macros;
mod options;
mod service;
mod subcmds;
mod utils;

fn err_exit(err: anyhow::Error) -> anyhow::Result<ExitCode> {
    writeln!(stderr(), "bite: error: {err}")?;
    Ok(ExitCode::from(2))
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // parse service options to determine the service type
    let (base, args) = match options::ServiceCommand::service() {
        Ok(value) => value,
        Err(e) => return err_exit(e),
    };

    // parse remaining args and run command
    let cmd = options::Command::parse_from(args);
    cmd.run(base).or_else(err_exit)
}
