use std::fs;
use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use bugbite::utils::current_dir;
use camino::Utf8PathBuf;
use clap::Args;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// list attachment metadata
    #[arg(short, long, conflicts_with = "view")]
    list: bool,

    /// output attachment data
    #[arg(short = 'V', long, conflicts_with = "dir")]
    view: bool,

    /// request attachments from bug IDs
    #[arg(short, long)]
    item_id: bool,

    /// save attachments to a specified directory
    #[arg(short, long, value_name = "PATH")]
    dir: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// attachment or bug IDs
    #[clap(help_heading = "Arguments")]
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let mut stdout = stdout().lock();
        let get_data = !self.options.list;
        let attachments = if self.options.item_id {
            async_block!(client.item_attachments(&self.ids, get_data))
        } else {
            async_block!(client.attachments(&self.ids, get_data))
        }?;

        let dir = self.options.dir.unwrap_or(current_dir()?);
        if self.options.list {
            for attachment in attachments {
                write!(stdout, "{attachment}")?;
            }
        } else if self.options.view {
            for attachment in attachments {
                // TODO: support auto-decompressing standard archive formats
                write!(stdout, "{}", attachment.read())?;
            }
        } else {
            fs::create_dir_all(&dir)?;
            for attachment in attachments {
                let path = dir.join(&attachment.file_name);
                // TODO: confirm overwriting file (with a -f/--force option?)
                if path.exists() {
                    anyhow::bail!("file already exists: {path}");
                }
                writeln!(stdout, "Saving attachment: {path}")?;
                fs::write(&path, attachment.data())?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
