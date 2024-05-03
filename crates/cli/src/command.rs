use std::io::stderr;
use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, ValueHint};
use clap_verbosity_flag::{LevelFilter, Verbosity, WarnLevel};
use tracing_log::AsTrace;

use crate::config::Config;
use crate::subcmds::Subcommand;
use crate::utils::{wrapped_doc, COLUMNS};

fn enable_logging(verbosity: LevelFilter) {
    // Simplify log output when using info level since bugbite uses it for information messages
    // that shouldn't be prefixed. The downside is warning and error level messages will also
    // be non-prefixed, but they shouldn't occur during info level runs in most situations.
    let format = if verbosity == LevelFilter::Info {
        tracing_subscriber::fmt::format()
            .with_level(false)
            .with_target(false)
            .without_time()
            .compact()
    } else {
        tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(true)
            .without_time()
            .compact()
    };

    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(verbosity.as_trace())
        .with_writer(stderr)
        .init();
}

#[derive(Debug, Parser)]
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
