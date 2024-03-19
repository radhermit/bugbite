use std::process::ExitCode;

use clap::Parser;

mod config;
mod macros;
mod options;
mod service;
mod subcmds;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // parse service options to determine the service type
    let (base, args) = options::ServiceCommand::service()?;
    // parse remaining args and run command
    let cmd = options::Command::parse_from(args);
    cmd.run(base)
}
