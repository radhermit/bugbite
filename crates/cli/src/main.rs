use std::process::ExitCode;

use clap::Parser;

mod config;
mod options;
mod service;
mod subcmds;
mod test;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // reset SIGPIPE behavior since rust ignores it by default
    utils::reset_sigpipe();

    // parse service options to determine the service type
    let (base, args, options) = options::ServiceCommand::service()?;
    // parse remaining args and run command
    let cmd = options::Command::parse_from(args);
    cmd.run(base, options).await
}
