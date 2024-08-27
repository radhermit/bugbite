use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args)]
pub(super) struct Command;

impl Command {
    pub(super) async fn run(&self, service: &Service) -> anyhow::Result<ExitCode> {
        let version = service.version().send().await?;
        let mut stdout = stdout().lock();
        writeln!(stdout, "{version}")?;
        Ok(ExitCode::SUCCESS)
    }
}
