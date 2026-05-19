use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args, Debug)]
#[clap(next_help_heading = "User options")]
struct Options {
    /// include disabled user accounts
    #[arg(short, long)]
    disabled: bool,

    /// user group IDs or names
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        num_args = 1
    )]
    groups: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// user IDs or names
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run<W>(&self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.iter().flatten();
        let mut request = service.user_get(ids).disabled(self.options.disabled);
        if let Some(values) = &self.options.groups {
            request = request.groups(values);
        }

        let users = request.send().await?;
        for user in users {
            writeln!(f, "{user}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
