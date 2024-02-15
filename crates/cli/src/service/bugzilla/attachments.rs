use std::fs;
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use bugbite::utils::current_dir;
use camino::Utf8PathBuf;
use clap::Args;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// output attachment data
    #[arg(short = 'V', long, conflicts_with = "dir")]
    view: bool,

    /// search by bug ID
    #[arg(short, long)]
    bug_id: bool,

    /// save attachments to a specified directory
    #[arg(short, long, value_name = "PATH")]
    dir: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// attachment IDs
    #[clap(help_heading = "Arguments")]
    ids: Vec<String>,
}

impl Command {
    pub(super) fn run(self, client: Client) -> anyhow::Result<ExitCode> {
        let dir = self.options.dir.unwrap_or(current_dir()?);
        let attachments = async_block!(client.attachments(&self.ids))?;
        if self.options.view {
            for attachment in attachments {
                // TODO: support auto-decompressing standard archive formats
                print!("{}", attachment.read());
            }
        } else {
            fs::create_dir_all(&dir)?;
            for attachment in attachments {
                let path = dir.join(&attachment.file_name);
                // TODO: confirm overwriting file (with a -f/--force option?)
                if path.exists() {
                    anyhow::bail!("file already exists: {path}");
                }
                println!("Saving attachment: {path}");
                fs::write(&path, attachment.data())?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
