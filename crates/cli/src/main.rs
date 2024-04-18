use std::env;
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

    let (base, options, cmd) = options::Command::parse_args(env::args())?;
    cmd.run(base, options).await
}
