use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use anyhow::anyhow;
use bugbite::config::Config;
use bugbite::service::bugzilla::{self, Bugzilla};
use bugbite::service::ServiceKind;
use bugbite::traits::Merge;
use clap::Args;
use tracing::debug;

mod attachment;
mod comment;
mod create;
mod fields;
mod get;
mod history;
mod search;
mod update;
mod version;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Authentication")]
struct Authentication {
    /// API key
    #[arg(short, long, env = "BUGBITE_KEY")]
    key: Option<String>,

    /// username
    #[arg(short, long, env = "BUGBITE_USER")]
    user: Option<String>,

    /// password
    #[arg(short, long, env = "BUGBITE_PASS")]
    password: Option<String>,
}

impl From<Authentication> for bugzilla::Authentication {
    fn from(value: Authentication) -> Self {
        Self {
            key: value.key,
            user: value.user,
            password: value.password,
        }
    }
}

#[derive(Args, Debug)]
pub(crate) struct Command {
    #[clap(flatten)]
    service: super::ServiceOptions,

    #[clap(flatten)]
    auth: Authentication,

    #[command(subcommand)]
    cmd: Subcommand,
}

impl Command {
    pub(crate) async fn run<W>(self, config: &Config, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        // load or create a service config
        let connection = self.service.connection(config)?;
        let mut config = config
            .get_kind(ServiceKind::Bugzilla, connection)?
            .into_bugzilla()
            .map_err(|_| anyhow!("incompatible connection: {connection}"))?;

        // cli options override config settings
        config.auth.merge(self.auth.into());
        config.client.merge(self.service.into());

        let service = config.into_service()?;
        debug!("Service: {service}");
        self.cmd.run(&service, f).await
    }
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    /// Attachment commands
    #[command(visible_alias = "a")]
    Attachment(Box<attachment::Command>),

    /// Get bug comments
    Comment(comment::Command),

    /// Create bug
    #[command(visible_alias = "c")]
    Create(Box<create::Command>),

    /// Get bugzilla fields
    Fields(fields::Command),

    /// Get bugs
    #[command(visible_alias = "g")]
    Get(get::Command),

    /// Get bug changes
    History(history::Command),

    /// Search bugs
    #[command(visible_alias = "s")]
    Search(Box<search::Command>),

    /// Update bugs
    #[command(visible_alias = "u")]
    Update(Box<update::Command>),

    /// Get bugzilla version
    Version(version::Command),
}

impl Subcommand {
    async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        match self {
            Self::Attachment(cmd) => cmd.run(service, f).await,
            Self::Comment(cmd) => cmd.run(service, f).await,
            Self::Create(cmd) => cmd.run(service, f).await,
            Self::Fields(cmd) => cmd.run(service, f).await,
            Self::Get(cmd) => cmd.run(service, f).await,
            Self::History(cmd) => cmd.run(service, f).await,
            Self::Search(cmd) => cmd.run(service, f).await,
            Self::Update(cmd) => cmd.run(service, f).await,
            Self::Version(cmd) => cmd.run(service, f).await,
        }
    }
}
