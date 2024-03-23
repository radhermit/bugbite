use std::fs;
use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use camino::Utf8PathBuf;
use clap::Args;

use crate::macros::async_block;
use crate::service::Render;
use crate::utils::COLUMNS;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attachments options")]
struct Options {
    /// list attachment metadata
    #[arg(short, long, conflicts_with_all = ["dir", "view"])]
    list: bool,

    /// output attachment data
    #[arg(short = 'V', long, conflicts_with_all = ["dir", "list"])]
    view: bool,

    /// request attachments from bug IDs or aliases
    #[arg(
        short,
        long,
        num_args = ..,
        conflicts_with = "ids",
        value_name = "IDS",
        long_help = indoc::indoc! {"
            Request all attachments from the specified bug IDs or aliases.

            Regular ID arguments relate to individual attachment IDs. Using this
            option pulls all attachments from the related bugs.

            Note that when saving multiple attachments from multiple bugs,
            subdirectories named after the bug IDs are automatically used in
            order to avoid file name overlap.
        "}
    )]
    item_ids: Option<Vec<MaybeStdinVec<String>>>,

    /// save attachments into a base directory
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_value = ".",
        long_help = indoc::indoc! {"
            Save attachments to a specified directory.

            By default, attachments are saved to the current working directory
            and this allows altering that target directory.
        "}
    )]
    dir: Utf8PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// attachment IDs
    #[clap(required = true, help_heading = "Arguments")]
    ids: Vec<MaybeStdinVec<u64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut stdout = stdout().lock();
        let get_data = !self.options.list;

        let (attachments, multiple_bugs) = if let Some(ids) = &self.options.item_ids {
            let ids = &ids.iter().flatten().collect::<Vec<_>>();
            let attachments = async_block!(client.item_attachment(ids, get_data))?;
            (attachments, ids.len() > 1)
        } else {
            let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();
            let attachments = async_block!(client.attachment(ids, get_data))?;
            (attachments, false)
        };

        if self.options.list {
            for attachment in attachments.iter().flatten() {
                attachment.render(&mut stdout, *COLUMNS)?;
            }
        } else if self.options.view {
            for attachment in attachments.iter().flatten() {
                // TODO: support auto-decompressing standard archive formats
                write!(stdout, "{}", attachment.read())?;
            }
        } else {
            let dir = &self.options.dir;
            fs::create_dir_all(dir)?;
            for attachment in attachments.iter().flatten() {
                // use per-bug directories when requesting attachments from multiple bugs
                let path = if multiple_bugs {
                    let dir = dir.join(attachment.bug_id.to_string());
                    fs::create_dir_all(&dir)?;
                    dir.join(&attachment.file_name)
                } else {
                    dir.join(&attachment.file_name)
                };

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
