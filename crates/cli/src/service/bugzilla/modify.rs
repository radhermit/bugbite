use std::num::NonZeroU64;
use std::process::ExitCode;
use std::str::FromStr;
use std::{fmt, fs};

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::modify::{Change, ModifyParams};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use tempfile::NamedTempFile;

use crate::macros::async_block;
use crate::utils::{confirm, launch_editor};

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
struct CustomField {
    name: String,
    // TODO: support array values
    value: String,
}

impl FromStr for CustomField {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if let Some((name, value)) = s.split_once('=') {
            Ok(Self {
                name: name.into(),
                value: value.into(),
            })
        } else {
            anyhow::bail!("invalid custom field: {s}")
        }
    }
}

impl fmt::Display for CustomField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

#[skip_serializing_none]
#[derive(Args, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// assign to a user
    #[arg(short, long, value_name = "USER")]
    assigned_to: Option<String>,

    /// add/remove/set blockers
    #[arg(short = 'B', long, num_args = 0..=1, value_delimiter = ',')]
    blocks: Option<Vec<Change<NonZeroU64>>>,

    /// add/remove CC users
    #[arg(long, value_delimiter = ',')]
    cc: Option<Vec<Change<String>>>,

    /// add a comment
    #[arg(short = 'c', long)]
    comment: Option<String>,

    /// modify component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// modify custom fields
    #[arg(short = 'f', long = "field", value_name = "NAME=VALUE")]
    custom_fields: Option<Vec<CustomField>>,

    /// add/remove/set dependencies
    #[arg(short = 'D', long, num_args = 0..=1, value_delimiter = ',')]
    depends_on: Option<Vec<Change<NonZeroU64>>>,

    /// mark bug as duplicate
    #[arg(short, long, value_name = "ID", conflicts_with_all = ["status", "resolution"])]
    duplicate_of: Option<NonZeroU64>,

    /// add/remove groups
    #[arg(short = 'G', long, value_delimiter = ',')]
    groups: Option<Vec<Change<String>>>,

    /// add/remove/set keywords
    #[arg(short = 'K', long, value_delimiter = ',')]
    keywords: Option<Vec<Change<String>>>,

    /// modify OS
    #[arg(long)]
    os: Option<String>,

    /// modify platform
    #[arg(long)]
    platform: Option<String>,

    /// modify priority
    #[arg(long)]
    priority: Option<String>,

    /// modify product
    #[arg(short = 'P', long)]
    product: Option<String>,

    /// modify resolution
    #[arg(short = 'R', long)]
    resolution: Option<String>,

    /// modify severity
    #[arg(long)]
    severity: Option<String>,

    /// modify status
    #[arg(short, long)]
    status: Option<String>,

    /// modify target
    #[arg(long)]
    target: Option<String>,

    /// modify summary
    #[arg(short, long)]
    title: Option<String>,

    /// modify URL
    #[arg(short = 'U', long)]
    url: Option<String>,

    /// modify version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// modify whiteboard
    #[arg(short = 'W', long)]
    whiteboard: Option<String>,
}

impl Options {
    /// Merge two Option structs together, prioritizing values from the first.
    fn merge(self, other: Self) -> Self {
        Self {
            assigned_to: self.assigned_to.or(other.assigned_to),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            comment: self.comment.or(other.comment),
            component: self.component.or(other.component),
            custom_fields: self.custom_fields.or(other.custom_fields),
            depends_on: self.depends_on.or(other.depends_on),
            duplicate_of: self.duplicate_of.or(other.duplicate_of),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
            product: self.product.or(other.product),
            resolution: self.resolution.or(other.resolution),
            status: self.status.or(other.status),
            severity: self.severity.or(other.severity),
            target: self.target.or(other.target),
            title: self.title.or(other.title),
            url: self.url.or(other.url),
            version: self.version.or(other.version),
            whiteboard: self.whiteboard.or(other.whiteboard),
        }
    }
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
        short = 'T',
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    template: Option<Utf8PathBuf>,

    /// write options to a template file
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    to_template: Option<Utf8PathBuf>,

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
                let template = toml::from_str(&data)
                    .map_err(|e| anyhow::anyhow!("failed parsing template: {path}: {e}"))?;
                // command-line options override template options
                options = options.merge(template);
            }
        };

        // write command-line options to a template file
        if let Some(path) = self.to_template.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&options)?;
                fs::write(path, data)?;
            }
        }

        if let Some(value) = options.assigned_to.as_ref() {
            params.assigned_to(value);
        }

        if let Some(values) = options.blocks {
            params.blocks(values);
        }

        if let Some(values) = options.cc {
            params.cc(values);
        }

        if let Some(value) = options.comment.as_ref() {
            params.comment(value);
        }

        if let Some(value) = options.component.as_ref() {
            params.component(value);
        }

        if let Some(values) = options.custom_fields {
            params.custom_fields(values.into_iter().map(|f| (f.name, f.value)));
        }

        if let Some(values) = options.depends_on {
            params.depends_on(values);
        }

        if let Some(value) = options.duplicate_of {
            params.duplicate_of(value);
        }

        if let Some(values) = options.groups {
            params.groups(values);
        }

        if let Some(values) = options.keywords {
            params.keywords(values);
        }

        if let Some(value) = options.os.as_ref() {
            params.os(value);
        }

        if let Some(value) = options.platform.as_ref() {
            params.platform(value);
        }

        if let Some(value) = options.priority.as_ref() {
            params.priority(value);
        }

        if let Some(value) = options.product.as_ref() {
            params.product(value);
        }

        if let Some(value) = options.resolution.as_ref() {
            params.resolution(value);
        }

        if let Some(value) = options.severity.as_ref() {
            params.severity(value);
        }

        if let Some(value) = options.status.as_ref() {
            params.status(value);
        }

        if let Some(value) = options.target.as_ref() {
            params.target(value);
        }

        if let Some(value) = options.title.as_ref() {
            params.summary(value);
        }

        if let Some(value) = options.url.as_ref() {
            params.url(value);
        }

        if let Some(value) = options.version.as_ref() {
            params.version(value);
        }

        if let Some(value) = options.whiteboard.as_ref() {
            params.whiteboard(value);
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
