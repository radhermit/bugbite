use std::io::stderr;
use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, ValueHint};
use clap_verbosity_flag::{LevelFilter, Verbosity, WarnLevel};
use tracing_log::AsTrace;

use crate::config::Config;
use crate::subcmds::Subcommand;
use crate::utils::{verbose, wrapped_doc, COLUMNS};

fn enable_logging(verbosity: LevelFilter) {
    // enable verbose output
    if verbosity >= LevelFilter::Info {
        verbose!(true);
    };

    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(true)
        .without_time()
        .compact();

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(verbosity.as_trace())
        .with_writer(stderr)
        .init();
}

#[derive(Parser)]
#[command(
    name = "bite",
    version,
    about = "command line tool for mangling bugs, issues, and tickets",
    disable_help_subcommand = true,
    term_width = *COLUMNS,
    help_template = wrapped_doc!("
        {before-help}{name} {version}

        {about}

        {usage-heading} {usage}

        {all-args}{after-help}
    ")
)]
pub(crate) struct Command {
    /// load config from a custom path
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    config: Option<Utf8PathBuf>,

    #[clap(flatten)]
    verbosity: Verbosity<WarnLevel>,

    #[command(subcommand)]
    subcmd: Subcommand,
}

impl Command {
    /// Run the command.
    pub(super) async fn run() -> anyhow::Result<ExitCode> {
        let cmd = Command::parse();
        enable_logging(cmd.verbosity.log_level_filter());

        // TODO: drop this once stable rust supports `unix_sigpipe`,
        // see https://github.com/rust-lang/rust/issues/97889.
        //
        // Reset SIGPIPE to the default behavior since rust ignores it by default.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }

        let config = Config::load(cmd.config.as_deref())?;
        cmd.subcmd.run(&config).await
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use bugbite::test::{build_path, reset_stdin, TestServer};

    use super::*;

    #[tokio::test]
    async fn doc() {
        // wipe bugbite-related environment variables
        for (key, _value) in env::vars() {
            if key.starts_with("BUGBITE_") {
                env::remove_var(key);
            }
        }

        // start mocked server
        let server = TestServer::new().await;
        env::set_var("BUGBITE_CONNECTION", server.uri());
        env::set_var("BUGBITE_USER", "bugbite@bugbite.test");
        env::set_var("BUGBITE_PASS", "bugbite");
        env::set_var("BUGBITE_KEY", "bugbite");

        let doc_dir = build_path!(env!("CARGO_MANIFEST_DIR"), "doc");
        for entry in doc_dir.read_dir_utf8().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map(|x| x == "adoc").unwrap_or_default() {
                let name = entry.file_name();
                let doc = fs::read_to_string(path).unwrap();
                for (lineno, line) in doc.lines().enumerate().filter(|(_, x)| x.starts_with(' ')) {
                    for cmd in line.trim().split(" | ").filter(|x| x.starts_with("bite ")) {
                        let args = shlex::split(cmd).unwrap();
                        if let Err(e) = Command::try_parse_from(args) {
                            panic!(
                                "failed parsing: {cmd}\nfile: {name}, line {}\n{e}",
                                lineno + 1
                            );
                        }
                        reset_stdin();
                    }
                }
            }
        }
    }
}
