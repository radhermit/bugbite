use std::hash::Hash;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::str::FromStr;
use std::{fmt, fs};

use anyhow::Context;
use bugbite::args::{MaybeStdin, MaybeStdinVec};
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::update::{Parameters, RangeOrSet, SetChange};
use bugbite::service::bugzilla::Service;
use bugbite::traits::{Merge, RequestSend, RequestTemplate};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use tempfile::NamedTempFile;

use crate::service::TemplateOptions;
use crate::utils::{confirm, launch_editor, prefix, verbose};

#[derive(Clone, Debug)]
struct CommentPrivacy<T: FromStr + PartialOrd + Eq + Hash> {
    raw: String,
    range_or_set: Option<RangeOrSet<T>>,
    is_private: Option<bool>,
}

impl<T: FromStr + PartialOrd + Eq + Hash> FromStr for CommentPrivacy<T>
where
    <T as FromStr>::Err: fmt::Display + fmt::Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (range_or_set, is_private) = if let Some((ids, value)) = s.split_once(':') {
            (Some(ids.parse()?), Some(value.parse()?))
        } else {
            (Some(s.parse()?), None)
        };

        Ok(Self {
            raw: s.to_string(),
            range_or_set,
            is_private,
        })
    }
}

impl<T: FromStr + PartialOrd + Eq + Hash> fmt::Display for CommentPrivacy<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.raw.fmt(f)
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Params {
    /// add/remove/set aliases
    #[arg(
        short = 'A',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
    )]
    alias: Option<Vec<SetChange<String>>>,

    /// update assignee
    #[arg(
        short,
        long,
        value_name = "USER",
        num_args = 0..=1,
        default_missing_value = "",
    )]
    assignee: Option<String>,

    /// add/remove/set blockers
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
    )]
    blocks: Option<Vec<SetChange<u64>>>,

    /// add/remove CC users
    #[arg(long, value_name = "USER[,...]", value_delimiter = ',')]
    cc: Option<Vec<SetChange<String>>>,

    /// add comment
    #[arg(
        short,
        long,
        num_args = 0..=1,
        conflicts_with_all = ["comment_from", "reply"],
        default_missing_value = "",
    )]
    comment: Option<MaybeStdin<String>>,

    /// add comment from file
    #[arg(
        short = 'F',
        long,
        conflicts_with_all = ["comment", "reply"],
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    comment_from: Option<Utf8PathBuf>,

    /// enable comment privacy
    #[arg(short = 'P', long, num_args = 0, default_missing_value = "true")]
    comment_is_private: Option<bool>,

    /// update comment privacy
    #[arg(long, value_name = "VALUE")]
    comment_privacy: Option<CommentPrivacy<usize>>,

    /// update component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// update custom field
    #[arg(long = "cf", num_args = 2, value_names = ["NAME", "VALUE"])]
    custom_fields: Option<Vec<String>>,

    /// add/remove/set dependencies
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
    )]
    depends: Option<Vec<SetChange<u64>>>,

    /// mark bug as duplicate
    #[arg(short = 'D', long, value_name = "ID", conflicts_with_all = ["status", "resolution"])]
    duplicate_of: Option<u64>,

    /// add/remove flags
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    flags: Option<Vec<Flag>>,

    /// add/remove groups
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    groups: Option<Vec<SetChange<String>>>,

    /// add/remove/set keywords
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
    )]
    keywords: Option<Vec<SetChange<String>>>,

    /// update operating system
    #[arg(long)]
    os: Option<String>,

    /// update platform
    #[arg(long)]
    platform: Option<String>,

    /// update priority
    #[arg(long)]
    priority: Option<String>,

    /// update product
    #[arg(short, long)]
    product: Option<String>,

    /// update QA contact
    #[arg(
        long,
        value_name = "USER",
        num_args = 0..=1,
        default_missing_value = "",
    )]
    qa: Option<String>,

    /// update resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// add/remove bug URLs
    #[arg(short = 'U', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    see_also: Option<Vec<SetChange<String>>>,

    /// update severity
    #[arg(long)]
    severity: Option<String>,

    /// update status
    #[arg(short, long)]
    status: Option<String>,

    /// update summary
    #[arg(short = 'S', long)]
    summary: Option<String>,

    /// update target milestone
    #[arg(short = 'T', long, value_name = "MILESTONE")]
    target: Option<String>,

    /// update URL
    #[arg(short, long)]
    url: Option<String>,

    /// update version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// update whiteboard
    #[arg(short, long)]
    whiteboard: Option<String>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            alias: value.alias,
            assignee: value.assignee,
            blocks: value.blocks,
            cc: value.cc,
            comment: value.comment.map(|x| x.into_inner()),
            comment_from: value.comment_from,
            comment_is_private: value.comment_is_private,
            comment_privacy: value
                .comment_privacy
                .and_then(|x| x.range_or_set.map(|value| (value, x.is_private))),
            component: value.component,
            depends: value.depends,
            duplicate_of: value.duplicate_of,
            flags: value.flags,
            groups: value.groups,
            keywords: value.keywords,
            os: value.os,
            platform: value.platform,
            priority: value.priority,
            product: value.product,
            qa: value.qa,
            resolution: value.resolution,
            see_also: value.see_also,
            status: value.status,
            severity: value.severity,
            target: value.target,
            summary: value.summary,
            url: value.url,
            version: value.version,
            whiteboard: value.whiteboard,

            custom_fields: value.custom_fields.map(|x| {
                x.into_iter()
                    .tuples()
                    .map(|(k, v)| (prefix!("cf_", k), v))
                    .collect()
            }),
        }
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Update options")]
pub(super) struct Options {
    /// reply to specific comments
    #[arg(
        short = 'R',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        conflicts_with_all = ["comment", "comment_from"],
    )]
    reply: Option<Vec<usize>>,
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    template: TemplateOptions,

    #[clap(flatten)]
    params: Params,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(help_heading = "Arguments", required_unless_present = "dry_run")]
    ids: Vec<MaybeStdinVec<String>>,
}

/// Interactively create a reply, pulling specified comments for pre-population.
async fn get_reply(
    service: &Service,
    id: &str,
    comment_ids: &mut Vec<usize>,
) -> anyhow::Result<String> {
    let comments = service
        .comment([id])
        .send()
        .await?
        .into_iter()
        .next()
        .expect("invalid comments response");
    if comments.is_empty() {
        anyhow::bail!("reply invalid, bug {id} has no comments")
    }

    // use the last comment if no IDs were specified
    if comment_ids.is_empty() {
        comment_ids.push(comments.len() - 1);
    }

    let mut data = vec![];
    for id in comment_ids {
        let Some(comment) = comments.get(*id) else {
            anyhow::bail!("reply invalid, nonexistent comment #{id}");
        };
        data.push(comment);
    }
    let data = data.iter().map(|x| x.reply()).join("\n\n");

    // interactively edit the comment
    edit_comment(&data)
}

/// Interactively edit a comment.
fn edit_comment(data: &str) -> anyhow::Result<String> {
    let temp_file = NamedTempFile::new()?;
    if !data.is_empty() {
        fs::write(&temp_file, data).context("failed saving comment file")?;
    }

    loop {
        let status = launch_editor(temp_file.path())?;
        if !status.success() {
            anyhow::bail!("failed editing comment");
        }
        let comment = fs::read_to_string(&temp_file).context("failed reading comment file")?;
        let comment = comment.trim().to_string();
        if comment != data || confirm("No changes made, submit anyway?", false)? {
            return Ok(comment);
        }
    }
}

impl Command {
    pub(super) async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let ids = self.ids.into_iter().flatten();
        let mut request = service.update(ids);

        // read attributes from template
        if let Some(name) = self.template.from.as_deref() {
            let params = request.load_template(name)?;
            request.params.merge(params);
        }

        // command line parameters override template
        request.params.merge(self.params.into());

        // write attributes to template
        if let Some(name) = self.template.to.as_deref() {
            request.save_template(name)?;
        }

        // interactively create reply or comment
        if let Some(mut values) = self.options.reply {
            if request.ids.len() != 1 {
                anyhow::bail!("reply must target a single bug");
            }
            let comment = get_reply(service, &request.ids[0], &mut values).await?;
            request.params.comment = Some(comment);
        } else if let Some(value) = request.params.comment.as_ref() {
            if value.trim().is_empty() {
                let comment = edit_comment(value.trim())?;
                request.params.comment = Some(comment);
            }
        }

        if !self.template.dry_run {
            let changes = request.send().await?;
            for change in changes {
                verbose!(f, "{change}")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
