use std::process::ExitCode;

mod command;
mod config;
mod service;
mod subcmds;
mod test;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    command::Command::run().await
}
