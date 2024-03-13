use std::fs;
use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::modify::ModifyParams;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::Deserialize;
use tempfile::NamedTempFile;

use crate::macros::async_block;
use crate::utils::{confirm, launch_editor};

#[derive(Debug, Args, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// modify status
    #[arg(short, long)]
    status: Option<String>,

    /// modify resolution
    #[arg(short = 'R', long)]
    resolution: Option<String>,

    /// mark bug as duplicate
    #[arg(short, long, value_name = "ID", conflicts_with_all = ["status", "resolution"])]
    duplicate_of: Option<NonZeroU64>,

    /// modify component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// modify product
    #[arg(short = 'P', long)]
    product: Option<String>,

    /// add a comment
    #[arg(short = 'c', long)]
    comment: Option<String>,

    /// modify summary
    #[arg(short, long)]
    title: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// reply to specific comment(s)
    #[arg(short, long, value_delimiter = ',', help_heading = "Modify options")]
    reply: Option<Vec<usize>>,

    /// load options from a template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    template: Option<Utf8PathBuf>,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs
    #[clap(
        required = true,
        help_heading = "Arguments",
        long_help = indoc::indoc! {"
            IDs of bugs to modify.

            Taken from standard input when `-`.
        "}
    )]
    ids: Vec<MaybeStdinVec<NonZeroU64>>,
}

impl Command {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();
        let mut options = self.options;
        let mut params = ModifyParams::new();

        // Try to load a template as modify parameters with a fallback to loading as modify options
        // on failure.
        if let Some(path) = self.template.as_ref() {
            if let Ok(value) = ModifyParams::load(path) {
                params = value;
            } else {
                let data = fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("failed loading template: {path}: {e}"))?;
                options = toml::from_str(&data)
                    .map_err(|e| anyhow::anyhow!("failed parsing template: {path}: {e}"))?;
            }
        };

        if let Some(value) = options.status.as_ref() {
            params.status(value);
        }
        if let Some(value) = options.resolution.as_ref() {
            params.resolution(value);
        }
        if let Some(value) = options.duplicate_of {
            params.duplicate_of(value);
        }
        if let Some(value) = options.component.as_ref() {
            params.component(value);
        }
        if let Some(value) = options.product.as_ref() {
            params.product(value);
        }
        if let Some(value) = options.comment.as_ref() {
            params.comment(value);
        }
        if let Some(value) = options.title.as_ref() {
            params.summary(value);
        }

        // pull comments to interactively create a reply
        if let Some(values) = self.reply.as_ref() {
            if ids.len() > 1 {
                anyhow::bail!("reply invalid, targeting multiple bugs");
            }
            let id = ids[0];

            let comments = async_block!(client.comments(&[id], None))?
                .into_iter()
                .next()
                .expect("invalid comments response");
            if comments.is_empty() {
                anyhow::bail!("reply invalid, bug #{id} has no comments")
            }

            let mut data = vec![];
            for i in values {
                let Some(comment) = comments.get(*i) else {
                    anyhow::bail!("reply invalid, nonexistent comment #{i}");
                };
                data.push(comment);
            }
            let data = data.iter().map(|x| x.reply()).join("\n\n");

            // interactively edit the comment
            let temp_file = NamedTempFile::new()?;
            fs::write(&temp_file, &data)?;
            loop {
                let status = launch_editor(&temp_file)?;
                if !status.success() {
                    anyhow::bail!("failed editing reply content");
                }
                let comment = fs::read_to_string(&temp_file)?;
                if comment != data || confirm("No changes made to comment, submit anyway?", false)?
                {
                    params.comment(comment.trim());
                    break;
                }
            }
        }

        async_block!(client.modify(ids, params))?;
        Ok(ExitCode::SUCCESS)
    }
}
