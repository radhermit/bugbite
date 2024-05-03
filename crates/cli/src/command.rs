use std::collections::HashSet;
use std::env;
use std::io::stderr;
use std::process::ExitCode;

use bugbite::service::{ClientBuilder, ServiceKind};
use camino::Utf8PathBuf;
use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, ValueHint};
use clap_verbosity_flag::{LevelFilter, Verbosity, WarnLevel};
use strum::VariantNames;
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
#[command(disable_help_flag = true)]
pub(super) struct ServiceCommand {
    #[clap(flatten)]
    options: Options,

    #[arg(
        num_args = ..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
    )]
    remaining: Vec<String>,
}

impl ServiceCommand {
    /// Try parsing arguments from a given source.
    pub(crate) fn try_parse_args<I, T>(
        args: I,
    ) -> clap::error::Result<(String, Vec<String>, Options)>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut args: Vec<_> = args.into_iter().map(Into::into).collect();
        let cmd = Self::try_parse_from(&args)?;
        let remaining = cmd.remaining;
        // pull the first, remaining argument
        let arg = remaining.first().map(|x| x.as_str()).unwrap_or_default();
        let subcmds: HashSet<_> = Subcommand::VARIANTS.iter().copied().collect();
        let services: HashSet<_> = ServiceKind::VARIANTS.iter().copied().collect();

        // early return for non-service subcommands
        if subcmds.contains(arg) && !services.contains(arg) {
            return Ok((Default::default(), args, cmd.options));
        }

        let config = Config::load(cmd.options.config.as_deref())
            .map_err(|e| Command::error(ErrorKind::InvalidValue, e))?;

        let Some(connection) = cmd.options.connection.as_deref() else {
            // handle -h/--help options
            if services.contains(arg) || arg.starts_with('-') {
                Command::try_parse_from(&args)?;
            }

            return Err(Command::error(
                ErrorKind::MissingRequiredArgument,
                "no connection specified",
            ));
        };

        // parse manual connection string
        let manual_connection = |value: &str| -> anyhow::Result<(ServiceKind, String)> {
            if let Some((service, base)) = value.split_once('@') {
                let kind = service
                    .parse()
                    .map_err(|_| anyhow::anyhow!("unknown service: {service}"))?;
                Ok((kind, base.to_string()))
            } else {
                anyhow::bail!("unknown connection: {value}")
            }
        };

        // pull from config with fallback to manual connection
        let (service, base) = config
            .get(connection)
            .or_else(|_| manual_connection(connection))
            .map_err(|e| Command::error(ErrorKind::InvalidValue, e))?;

        if services.contains(arg) && service.as_ref() != arg {
            let msg = format!("{arg} not compatible with connection service: {service}");
            return Err(Command::error(ErrorKind::InvalidValue, msg));
        }

        // construct new args for the main command to parse
        args.drain(1..);

        // inject subcommand for requested service type if missing
        if !subcmds.contains(arg) {
            args.push(service.to_string());
        }

        // append the remaining unparsed args
        args.extend(remaining);

        Ok((base, args, cmd.options))
    }
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    /// load config from a custom path
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    config: Option<Utf8PathBuf>,

    /// service connection
    #[arg(short, long, env = "BUGBITE_CONNECTION")]
    connection: Option<String>,

    /// ignore invalid service certificates
    #[arg(long)]
    insecure: bool,

    /// request timeout in seconds
    #[arg(short, long, value_name = "SECONDS", default_value = "30")]
    timeout: f64,
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
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    verbosity: Verbosity<WarnLevel>,

    #[command(subcommand)]
    subcmd: Subcommand,
}

impl Command {
    /// Run the command.
    pub(super) async fn run() -> anyhow::Result<ExitCode> {
        match Self::try_parse_args(env::args()) {
            Ok((base, options, cmd)) => {
                enable_logging(cmd.verbosity.log_level_filter());

                let builder = ClientBuilder::default()
                    .insecure(options.insecure)
                    .timeout(options.timeout);

                // TODO: drop this once stable rust supports `unix_sigpipe`,
                // see https://github.com/rust-lang/rust/issues/97889.
                //
                // Reset SIGPIPE to the default behavior since rust ignores it by default.
                unsafe {
                    libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                }

                cmd.subcmd.run(base, builder).await
            }
            Err(e) => e.exit(),
        }
    }

    /// Create a custom clap error.
    fn error(kind: ErrorKind, message: impl std::fmt::Display) -> clap::error::Error {
        Self::command().error(kind, message)
    }

    /// Try parsing arguments from a given source.
    pub(crate) fn try_parse_args<I, T>(args: I) -> clap::error::Result<(String, Options, Self)>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        // parse service options to determine the service type
        let (base, args, options) = ServiceCommand::try_parse_args(args)?;
        // parse remaining args
        let cmd = Self::try_parse_from(args)?;
        Ok((base, options, cmd))
    }
}
