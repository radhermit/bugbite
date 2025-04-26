use std::fs;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{Context, anyhow};
use bugbite::args::MaybeStdinVec;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::traits::RequestSend;
use camino::Utf8PathBuf;
use clap::Args;
use itertools::Itertools;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attachment options")]
struct Options {
    /// list attachment metadata
    #[arg(short, long, conflicts_with_all = ["dir", "output"])]
    list: bool,

    /// output attachment data
    #[arg(
        short,
        long,
        conflicts_with_all = ["dir", "list"],
        value_name = "FILE",
    )]
    output: Option<String>,

    /// include outdated attachments
    #[arg(short = 'O', long)]
    outdated: bool,

    /// request attachments from bug IDs or aliases
    #[arg(short, long)]
    item_ids: bool,

    /// save attachments into a base directory
    #[arg(
        short,
        long,
        value_name = "PATH",
        default_value = ".",
        conflicts_with_all = ["list", "output"],
    )]
    dir: Utf8PathBuf,
}

#[derive(Args, Debug)]
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
    pub(super) async fn run<W>(&self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let multiple_bugs = self.options.item_ids && self.ids.len() > 1;
        let ids = self.ids.iter().flatten();

        let attachments = if self.options.item_ids {
            service
                .attachment_get_item(ids)
                .data(!self.options.list)
                .outdated(self.options.outdated)
                .send()
                .await?
                .into_iter()
                .flatten()
                .collect()
        } else {
            // convert IDs to numeric values
            let ids: Vec<_> = ids
                .map(|x| x.parse().map_err(|_| anyhow!("invalid attachment ID: {x}")))
                .try_collect()?;

            service
                .attachment_get(ids)
                .data(!self.options.list)
                .send()
                .await?
        };

        // conditionally skip deleted and obsolete attachments
        let attachments = attachments
            .iter()
            .filter(|x| self.options.outdated || (!x.is_obsolete && !x.is_deleted()));

        if self.options.list {
            for attachment in attachments {
                write!(f, "{attachment}")?;
            }
        } else if let Some(name) = self.options.output.as_deref() {
            for attachment in attachments {
                if name == "-" {
                    f.write_all(attachment.as_ref())
                        .context("failed writing to standard output")?;
                } else {
                    fs::write(name, attachment).context("failed writing to file: {name}")?;
                }
            }
        } else {
            let dir = &self.options.dir;
            fs::create_dir_all(dir).context("failed creating attachments directory")?;
            for attachment in attachments {
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

                writeln!(f, "Saving attachment: {path}")?;
                fs::write(&path, attachment).context("failed saving attachment")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
