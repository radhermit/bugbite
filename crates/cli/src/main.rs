use std::io::stderr;
use std::process::ExitCode;

use tracing::info;
use tracing_log::AsTrace;

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

    // parse initial options to determine the service type
    let service = match options::Command::service() {
        Ok(service) => service,
        Err(e) => return err_exit(e),
    };

    // re-parse all args using the service parser
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
    cmd.run(service).or_else(err_exit)
}
