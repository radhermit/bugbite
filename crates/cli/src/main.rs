use std::process::ExitCode;

use clap::Parser;

mod config;
mod macros;
mod options;
mod service;
mod subcmds;
mod utils;

fn err_exit(err: anyhow::Error) -> anyhow::Result<ExitCode> {
    eprintln!("bite: error: {err}");
    Ok(ExitCode::from(2))
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // reset SIGPIPE behavior since rust ignores it by default
    utils::reset_sigpipe();

    let config = config::Config::default();
    // TODO: load user config that overrides defaults

    // parse service options to determine the service type
    let (kind, base, args) = match options::ServiceCommand::service(&config) {
        Ok(value) => value,
        Err(e) => return err_exit(e),
    };

    // parse remaining args and run command
    let cmd = options::Command::parse_from(args);
    cmd.run(kind, base).or_else(err_exit)
}
