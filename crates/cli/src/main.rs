use std::process::ExitCode;

mod command;
mod config;
mod service;
mod subcmds;
mod test;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // reset SIGPIPE behavior since rust ignores it by default
    utils::reset_sigpipe();
    command::Command::run().await
}
