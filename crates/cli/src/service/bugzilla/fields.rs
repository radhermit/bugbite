use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::service::bugzilla::Service;
use bugbite::traits::RequestSend;
use clap::Args;

#[derive(Args)]
pub(super) struct Command;

impl Command {
    pub(super) async fn run(&self, service: &Service) -> anyhow::Result<ExitCode> {
        let fields = service.fields().send().await?;
        let mut stdout = stdout().lock();
        for field in &fields {
            writeln!(stdout, "{field}\n")?;
        }
        Ok(ExitCode::SUCCESS)
    }
}
