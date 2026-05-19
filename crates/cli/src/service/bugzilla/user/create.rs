use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args, Debug)]
#[clap(next_help_heading = "User options")]
struct Options {
    /// real name
    #[arg(short, long)]
    name: Option<String>,

    /// password
    #[arg(short, long)]
    password: Option<String>,
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// user emails
    #[clap(required = true, help_heading = "Arguments")]
    emails: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run<W>(&self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let emails = self.emails.iter().flatten();
        let request = service
            .user_create(emails)
            .name(self.options.name.as_deref())
            .password(self.options.password.as_deref());

        let ids = request.send().await?;
        for id in ids {
            writeln!(f, "{id}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
