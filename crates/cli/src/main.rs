use std::process::ExitCode;

mod command;
mod service;
mod subcmds;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    command::Command::run().await
}
