use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::service::bugzilla::user::update::*;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args, Debug)]
#[clap(next_help_heading = "User options")]
struct Params {
    /// email address
    #[arg(short, long)]
    email: Option<String>,

    /// real name
    #[arg(short, long)]
    name: Option<String>,

    /// password
    #[arg(short, long)]
    password: Option<String>,

    /// toggle sending bug-related email
    #[arg(short = 'E', long, value_name = "BOOL")]
    email_enabled: Option<bool>,

    /// disable user account with reason
    #[arg(short, long, value_name = "REASON")]
    disable: Option<String>,
    // TODO: add groups support
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            email: value.email,
            name: value.name,
            password: value.password,
            email_enabled: value.email_enabled,
            disable: value.disable,
        }
    }
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// user IDs or names
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.iter().flatten();
        let mut request = service.user_update(ids);
        request.params = self.params.into();

        let users = request.send().await?;
        for user in users {
            writeln!(f, "{user}")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
