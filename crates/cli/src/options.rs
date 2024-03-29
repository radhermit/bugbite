use std::collections::HashSet;
use std::env;
use std::io::stderr;
use std::process::ExitCode;

use bugbite::client::Client;
use bugbite::service::ServiceKind;
use bugbite::services::SERVICES;
use camino::Utf8PathBuf;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, Parser, ValueHint};
use clap_verbosity_flag::{LevelFilter, Verbosity, WarnLevel};
use itertools::Itertools;
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
    pub(crate) fn service() -> anyhow::Result<(String, Vec<String>)> {
        // parse service options
        let Ok(cmd) = Self::try_parse() else {
            // use main command parser if first arg is an option (e.g. --help or --version)
            if env::args()
                .nth(1)
                .map(|x| x.starts_with('-'))
                .unwrap_or(true)
            {
                Command::parse();
            }
            // fallback to service parser to handle service restriction failures
            Self::parse();
            anyhow::bail!("failed parsing service options");
        };

        let remaining = cmd.remaining;
        // pull the first, remaining argument
        let arg = remaining.first().map(|x| x.as_str()).unwrap_or_default();
        let subcmds: HashSet<_> = Subcommand::VARIANTS.iter().copied().collect();
        let services: HashSet<_> = ServiceKind::VARIANTS.iter().copied().collect();

        // early return for non-service subcommands
        if subcmds.contains(arg) && !services.contains(arg) {
            return Ok((Default::default(), env::args().collect()));
        }

        let config = Config::load(cmd.options.bite.config.as_deref())?;
        let connection = cmd.options.service.connection.as_deref();
        let base = cmd.options.service.base.as_deref();
        let service = cmd.options.service.service;

        // determine service type
        let (selected, base) = match (connection, base, service) {
            (Some(name), _, _) => {
                let (kind, base) = config.get(name)?;
                if services.contains(arg) && kind.as_ref() != arg {
                    anyhow::bail!("{arg} not compatible with connection: {name}");
                }
                (kind, base)
            }
            (None, Some(base), Some(service)) => (service, base.to_string()),
            _ => {
                if services.contains(arg) || arg.starts_with('-') {
                    Command::parse();
                }

                anyhow::bail!("no connection specified");
            }
        };

        // construct new args for the main command to parse
        let mut args = vec![env::args().next().unwrap_or_default()];

        // inject subcommand for requested service type if missing
        if !subcmds.contains(arg) {
            args.push(selected.to_string());
        }

        // append the remaining unparsed args
        args.extend(remaining);

        Ok((base, args))
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Service")]
struct ServiceOpts {
    /// use pre-configured connection
    #[arg(
        short,
        long,
        env = "BUGBITE_CONNECTION",
        long_help = wrapped_doc!("
            Use a pre-configured connection by its alias.

            Connections can be defined in the user config. The precedence order
            when overlapping connection names exist or when multiple locations
            are defined is as follows from lowest to highest: internal, user
            config, environment, and command option. Specifying a connection
            always overrides the manual --base and --service settings.

            The following connections are defined internally in bugbite
            for ease of use:

            {}

            It's also possible to specify a target connection via the
            environment variable seen below.",
            SERVICES.iter()
                .map(|(name, config)| format!("{name}: {config}"))
                .sorted().join("\n")
        )
    )]
    connection: Option<String>,
    /// base service URL
    #[arg(
        short,
        long,
        env = "BUGBITE_BASE",
        long_help = wrapped_doc!("
            Specify the service URL to connect to.

            For example, a bugzilla service would use `https://bugzilla.kernel.org`
            and a github service would use `https://github.com/radhermit/bugbite`.
        ")
    )]
    base: Option<String>,
    /// service type
    #[arg(
        short,
        long,
        env = "BUGBITE_SERVICE",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
            .map(|s| s.parse::<ServiceKind>().unwrap()),
        long_help = wrapped_doc!("
            Specify the service type to use.

            Possible values: {}",
            ServiceKind::VARIANTS.join(", ")
        )
    )]
    service: Option<ServiceKind>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Bite options")]
struct BiteOpts {
    /// load config from a custom path
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    config: Option<Utf8PathBuf>,
    /// ignore invalid service certificates
    #[arg(long)]
    insecure: bool,
    /// request timeout in seconds
    #[arg(short, long, value_name = "SECONDS", default_value = "30")]
    timeout: u64,
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[clap(flatten)]
    bite: BiteOpts,
    #[clap(flatten)]
    service: ServiceOpts,
}

#[derive(Debug, Parser)]
#[command(
    name = "bite",
    version,
    author = clap::crate_authors!(),
    about = "command line tool for mangling bugs, issues, and tickets",
    long_about = wrapped_doc!("
        Bite is a command line tool that aids interaction with a subset of the
        myriad bug, issue, and ticket trackers accessible online. It tries to
        support a consistent interface to search, request, modify, and create
        bugs (or their variants) in addition to other actions a service
        provides access to.
    "),
    disable_help_subcommand = true,
    term_width = *COLUMNS,
    help_template = wrapped_doc!("
        {before-help}{name} {version}

        {about}

        {usage-heading} {usage}

        Bite automatically injects service subcommands so they shouldn't be
        specified for quicker command-line access if desired. In general they
        aren't necessary except when trying to view the service specific help
        options via -h/--help. In addition, common service action subcommands
        are aliased to their first letter.

        {all-args}{after-help}
    ")
)]
pub(crate) struct Command {
    #[clap(flatten)]
    verbosity: Verbosity<WarnLevel>,

    #[clap(flatten)]
    options: Options,

    #[command(subcommand)]
    subcmd: Subcommand,
}

impl Command {
    pub(super) async fn run(self, base: String) -> anyhow::Result<ExitCode> {
        enable_logging(self.verbosity.log_level_filter());

        let client = Client::builder()
            .insecure(self.options.bite.insecure)
            .timeout(self.options.bite.timeout);

        self.subcmd.run(base, client).await
    }
}
