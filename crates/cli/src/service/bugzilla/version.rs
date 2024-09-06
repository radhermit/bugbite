use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args, Debug)]
pub(super) struct Command;

impl Command {
    pub(super) async fn run<W>(&self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let version = service.version().send().await?;
        writeln!(f, "{version}")?;
        Ok(ExitCode::SUCCESS)
    }
}
