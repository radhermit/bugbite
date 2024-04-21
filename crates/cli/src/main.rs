use std::process::ExitCode;

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
    options::Command::run().await
}
