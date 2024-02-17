use std::collections::{HashMap, HashSet};
use std::env;
use std::io::stderr;
use std::process::ExitCode;

use bugbite::client::Client;
use bugbite::service::ServiceKind;
use bugbite::services::SERVICES;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use itertools::Itertools;
use strum::{IntoEnumIterator, VariantNames};
use tracing_log::AsTrace;

use crate::subcmds;
use crate::utils::COLUMNS;

#[derive(Debug, Parser)]
#[command(disable_help_flag = true)]
pub(super) struct ServiceCommand {
    #[clap(flatten)]
    options: Options,
    #[command(subcommand)]
    subcmd: Remaining,
}

#[derive(Debug, Subcommand)]
enum Remaining {
    #[command(external_subcommand)]
    Args(Vec<String>),
}

impl ServiceCommand {
    pub(crate) fn service() -> anyhow::Result<(ServiceKind, String, Vec<String>)> {
        // parse pre-subcommand options with main command parsing failure fallback
        let Ok(cmd) = Self::try_parse() else {
            Command::parse();
            panic!("command parsing should have exited");
        };

        // create mapping for service kind to subcommand names
        let subcmds: HashMap<ServiceKind, String> = ServiceKind::iter()
            .map(|x| match x.as_ref().split_once('-') {
                Some(vals) => (x, vals.0.into()),
                None => (x, x.to_string()),
            })
            .collect();

        let connection = &cmd.options.service.connection;
        let base = &cmd.options.service.base;
        let service = &cmd.options.service.service;
        let Remaining::Args(remaining) = cmd.subcmd;
        let subcmd = remaining.first().map(|s| s.as_str()).unwrap_or_default();
        let subcmd_kind = subcmds
            .iter()
            .find(|(_, v)| v.as_str() == subcmd)
            .map(|(k, _)| k);

        // determine service type
        let (selected, base) = match (connection, base, service) {
            (Some(name), None, None) => SERVICES.get_raw(name)?,
            (None, Some(base), Some(service)) => (*service, base.clone()),
            (None, Some(base), None) => (ServiceKind::default(), base.clone()),
            (None, None, None) => match subcmd_kind {
                // TODO: use default service for type from user config if it exists
                Some(kind) => match SERVICES
                    .iter()
                    .find(|(_name, config)| config.kind() == *kind)
                {
                    Some((_name, config)) => (config.kind(), config.base().to_string()),
                    None => (*kind, "default".to_string()),
                },
                // TODO: use default service from user config if it exists
                None => SERVICES.get_raw("gentoo")?,
            },
            _ => panic!("misconfigured service option restrictions"),
        };

        // construct new args for the main command to parse
        let mut args: Vec<_> = env::args().take_while(|x| x != subcmd).collect();

        if let Some(kind) = subcmd_kind {
            if kind != &selected {
                // output help in case `-h/--help` is specified
                Command::parse();
                anyhow::bail!("{subcmd} not compatible with service: {selected}");
            }
        } else {
            // determine possible subcommands
            let possible_subcmds: HashSet<_> =
                subcmds::Subcommand::VARIANTS.iter().copied().collect();
            // inject subcommand for requested service type if missing
            if !possible_subcmds.contains(subcmd) {
                let cmd = subcmds.get(&selected).unwrap();
                args.push(cmd.clone());
            }
        }

        args.extend(remaining);
        Ok((selected, base, args))
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
        conflicts_with_all = ["base", "service"],
        long_help = indoc::formatdoc! {"
            Use a pre-configured connection by its alias.

            Connections can be defined in the user config and bugbite bundles
            many for well known projects (and bugbite itself). Note that user
            defined service aliases will take priority over bundled variants.

            Bundled services: {}
        ", SERVICES.iter().map(|(name, _)| name).sorted().join(", ")}
    )]
    connection: Option<String>,
    /// base service URL
    #[arg(
        short,
        long,
        env = "BUGBITE_BASE",
        long_help = indoc::indoc! {"
            Specify the service URL to connect to.

            For example, a bugzilla service would use `https://bugzilla.kernel.org`
            and a github service would use `https://github.com/radhermit/bugbite`.
        "}
    )]
    base: Option<String>,
    /// service type
    #[arg(
        short,
        long,
        env = "BUGBITE_SERVICE",
        requires = "base",
        long_help = indoc::formatdoc! {"
            Specify the service type to use.

            Possible values: {}
        ", ServiceKind::VARIANTS.join(", ")},
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(ServiceKind::VARIANTS)
            .map(|s| s.parse::<ServiceKind>().unwrap()),
    )]
    service: Option<ServiceKind>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Connection")]
struct Connection {
    /// skip SSL certificate verification
    #[arg(short = 'k', long)]
    insecure: bool,
    /// max number of concurrent requests
    #[arg(long)]
    concurrent: Option<usize>,
    /// seconds to wait before request timeout
    #[arg(short, long, value_name = "SECONDS", default_value = "30")]
    timeout: u64,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// skip service authentication
    #[arg(short = 'S', long)]
    skip: bool,
    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,
    /// password
    #[arg(short, long, env = "BUGBITE_PASS")]
    password: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct Options {
    #[command(flatten)]
    verbosity: Verbosity,
    #[clap(flatten)]
    service: ServiceOpts,
    #[clap(flatten)]
    connection: Connection,
    #[clap(flatten)]
    auth: Authentication,
}

#[derive(Debug, Parser)]
#[command(
    name = "bite",
    version,
    author = clap::crate_authors!(),
    about = "command line tool for mangling bugs, issues, and tickets",
    long_about = indoc::indoc! {"
        Bite is a command line tool that aids interaction with a subset of the
        myriad bug, issue, and ticket trackers accessible online. It tries to
        support a consistent interface to search, request, modify, and create
        bugs (or their variants) in addition to other actions a service
        provides access to.
    "},
    disable_help_subcommand = true,
    term_width = *COLUMNS,
    help_template = indoc::indoc! {"
        {before-help}{name} {version}
        {author}

        {about}

        {usage-heading} {usage}

        Bite automatically injects service subcommands so they can be dropped for quicker
        command-line access if desired. In addition, most common service action subcommands can be
        run by their aliases consisting of their first letter. For example, the command `bite
        bugzilla search test` is equivalent to `bite s test` when targeting the default bugzilla
        connection.

        {all-args}{after-help}
    "},
)]
pub(crate) struct Command {
    #[clap(flatten)]
    options: Options,

    #[command(subcommand)]
    subcmd: subcmds::Subcommand,
}

impl Command {
    pub(super) fn run(self, kind: ServiceKind, base: String) -> anyhow::Result<ExitCode> {
        // custom log event formatter
        let format = tracing_subscriber::fmt::format()
            .with_level(true)
            .with_target(false)
            .without_time()
            .compact();

        tracing_subscriber::fmt()
            .event_format(format)
            .with_max_level(self.options.verbosity.log_level_filter().as_trace())
            .with_writer(stderr)
            .init();

        let client = Client::builder()
            .insecure(self.options.connection.insecure)
            .timeout(self.options.connection.timeout);

        self.subcmd.run(kind, base, client)
    }
}
