use std::process::ExitCode;

use clap::Parser;

mod options;
mod service;
mod utils;

fn err_exit(err: anyhow::Error) -> anyhow::Result<ExitCode> {
    let cmd = env!("CARGO_BIN_NAME");
    eprintln!("{cmd}: error: {err}");
    Ok(ExitCode::from(2))
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // TODO: reset SIGPIPE behavior since rust ignores it by default

    // parse service options to determine the service type
    let (service, args) = match options::ServiceCommand::service() {
        Ok(value) => value,
        Err(e) => return err_exit(e),
    };

    // parse remaining args and run command
    let cmd = options::Command::parse_from(args);
    cmd.run(service).or_else(err_exit)
}
