use std::io::stderr;
use std::process::ExitCode;

use tracing::info;
use tracing_log::AsTrace;

mod options;
mod service;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    // TODO: reset SIGPIPE behavior since rust ignores it by default

    let service = options::Command::service()?;
    let cmd = service::Command::parse(&service);

    // custom log event formatter
    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(false)
        .without_time()
        .compact();

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(cmd.verbosity().log_level_filter().as_trace())
        .with_writer(stderr)
        .init();

    info!("{service}");
    cmd.run(service).or_else(|err| {
        eprintln!("bite: error: {err}");
        Ok(ExitCode::from(2))
    })
}
