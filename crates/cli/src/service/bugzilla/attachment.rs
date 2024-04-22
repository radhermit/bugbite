use std::fs;
use std::io::{stdout, Write};
use std::process::ExitCode;

use anyhow::Context;
use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use camino::Utf8PathBuf;
use clap::Args;

use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachment options")]
struct Options {
    /// list attachment metadata
    #[arg(short, long, conflicts_with_all = ["dir", "output"])]
    list: bool,

    /// output attachment data
    #[arg(
        short,
        long,
        conflicts_with_all = ["dir", "list", "item_ids"],
        value_name = "FILE",
    )]
    output: Option<String>,

    /// request attachments from bug IDs or aliases
    #[arg(short, long)]
    item_ids: bool,

    /// save attachments into a base directory
    #[arg(short, long, value_name = "PATH", default_value = ".")]
    dir: Utf8PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// attachment IDs or bug IDs/aliases
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<String>>,
}

impl Command {
    pub(super) async fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();
        let mut stdout = stdout().lock();

        let get_data = !self.options.list;
        let multiple_bugs = self.options.item_ids && ids.len() > 1;
        let attachments = client
            .attachment(ids, self.options.item_ids, get_data)
            .await?;

        if self.options.list {
            for attachment in attachments.iter().flatten() {
                attachment.render(&mut stdout, *COLUMNS)?;
            }
        } else if let Some(name) = self.options.output.as_deref() {
            if let Some(attachment) = attachments.iter().flatten().next() {
                let data = attachment.read()?;
                if name == "-" {
                    write!(stdout, "{data}")?;
                } else {
                    fs::write(name, &data).context("failed writing file")?;
                }
            }
        } else {
            let dir = &self.options.dir;
            fs::create_dir_all(dir).context("failed creating attachments directory")?;
            for attachment in attachments.iter().flatten() {
                // use per-bug directories when requesting attachments from multiple bugs
                let path = if multiple_bugs {
                    let dir = dir.join(attachment.bug_id.to_string());
                    fs::create_dir_all(&dir).context("failed creating attachments directory")?;
                    dir.join(&attachment.file_name)
                } else {
                    dir.join(&attachment.file_name)
                };

                // TODO: confirm overwriting file (with a -f/--force option?)
                if path.exists() {
                    anyhow::bail!("file already exists: {path}");
                }

                writeln!(stdout, "Saving attachment: {path}")?;
                fs::copy(attachment.path()?, &path).context("failed saving attachment")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc("bite-bugzilla-attachment");
    }
}
