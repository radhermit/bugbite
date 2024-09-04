use std::env;
use std::io::stdout;
use std::process::ExitCode;

use anyhow::anyhow;
use bugbite::config::Config;
use camino::Utf8PathBuf;
use strum::VariantNames;

use crate::service::*;

mod show;

#[derive(VariantNames, clap::Subcommand)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum Subcommand {
    // service subcommands
    /// bugzilla service support
    Bugzilla(bugzilla::Command),
    /// github service support
    Github(github::Command),
    /// redmine service support
    Redmine(redmine::Command),

    // regular subcommands
    /// show service information
    Show(show::Command),
}

impl Subcommand {
    pub(crate) async fn run(self) -> anyhow::Result<ExitCode> {
        let mut config = Config::new();

        // determine user config directory
        let config_dir =
            dirs_next::config_dir().ok_or_else(|| anyhow!("failed getting config directory"))?;
        let config_dir = Utf8PathBuf::from_path_buf(config_dir)
            .map_err(|e| anyhow!("invalid bugbite config directory: {e:?}"))?
            .join("bugbite");

        // load custom user services
        match env::var("BUGBITE_CONFIG").as_deref() {
            Err(_) if config_dir.exists() => config.load(config_dir)?,
            Ok("false") | Err(_) => (),
            Ok(path) => config.load(path)?,
        }

        let mut stdout = stdout().lock();
        match self {
            Self::Bugzilla(cmd) => cmd.run(&config, &mut stdout).await,
            Self::Github(cmd) => cmd.run(&config, &mut stdout).await,
            Self::Redmine(cmd) => cmd.run(&config, &mut stdout).await,
            Self::Show(cmd) => cmd.run(&config, &mut stdout),
        }
    }
}
