use std::fs;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Context;
use bugbite::args::MaybeStdinVec;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::create::Parameters;
use bugbite::service::bugzilla::Service;
use bugbite::traits::{Merge, RequestSend};
use bugbite::utils::is_terminal;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;

use crate::utils::{confirm, prefix, verbose};

#[derive(Args)]
#[clap(next_help_heading = "Attribute options")]
struct Params {
    /// set aliases
    #[arg(short = 'A', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    alias: Option<Vec<String>>,

    /// set assignee
    #[arg(short, long, value_name = "USER")]
    assignee: Option<String>,

    /// set blockers
    #[arg(short, long, value_name = "ID[,...]", value_delimiter = ',')]
    blocks: Option<Vec<MaybeStdinVec<String>>>,

    /// set CC users
    #[arg(long, value_name = "USER[,...]", value_delimiter = ',')]
    cc: Option<Vec<String>>,

    /// set component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// set custom field
    #[arg(
        long = "cf",
        num_args = 2,
        value_names = ["NAME", "VALUE"],
    )]
    custom_fields: Option<Vec<String>>,

    /// set dependencies
    #[arg(short, long, value_name = "ID[,...]", value_delimiter = ',')]
    depends: Option<Vec<MaybeStdinVec<String>>>,

    /// set description
    #[arg(short = 'D', long)]
    description: Option<String>,

    /// set flags
    #[arg(short = 'F', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    flags: Option<Vec<Flag>>,

    /// set groups
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
    )]
    groups: Option<Vec<String>>,

    /// set keywords
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    keywords: Option<Vec<String>>,

    /// set operating system
    #[arg(long)]
    os: Option<String>,

    /// set platform
    #[arg(long)]
    platform: Option<String>,

    /// set priority
    #[arg(long)]
    priority: Option<String>,

    /// set product
    #[arg(short, long)]
    product: Option<String>,

    /// set QA contact
    #[arg(long, value_name = "USER")]
    qa: Option<String>,

    /// set resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// set external bug URLs
    #[arg(short = 'U', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    see_also: Option<Vec<String>>,

    /// set severity
    #[arg(long)]
    severity: Option<String>,

    /// set status
    #[arg(short, long)]
    status: Option<String>,

    /// set summary
    #[arg(short = 'S', long)]
    summary: Option<String>,

    /// set target milestone
    #[arg(short = 'T', long, value_name = "MILESTONE")]
    target: Option<String>,

    /// set URL
    #[arg(short = 'u', long)]
    url: Option<String>,

    /// set version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// set whiteboard
    #[arg(short, long)]
    whiteboard: Option<String>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            alias: value.alias,
            assignee: value.assignee,
            blocks: value.blocks.map(|x| x.into_iter().flatten().collect()),
            cc: value.cc,
            component: value.component,
            depends: value.depends.map(|x| x.into_iter().flatten().collect()),
            description: value.description,
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

#[derive(Args)]
#[clap(next_help_heading = "Create options")]
pub(super) struct Options {
    /// skip service interaction
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// read attributes from template
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        conflicts_with = "from_bug",
    )]
    from: Option<Utf8PathBuf>,

    /// read attributes from an existing bug
    #[arg(long, value_name = "ID", conflicts_with = "from")]
    from_bug: Option<String>,

    /// write attributes to template
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    to: Option<Utf8PathBuf>,
}

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let mut request = service.create();

        // merge attributes from template or bug
        if let Some(path) = self.options.from.as_deref() {
            request.params.merge(path)?;
        } else if let Some(id) = self.options.from_bug {
            let bug = service
                .get([id])
                .send()
                .await?
                .into_iter()
                .next()
                .expect("failed getting bug");
            request.params.merge(bug)?;
        }

        // command line parameters override template
        request.params.merge(self.params)?;

        // write attributes to template
        if let Some(path) = self.options.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&request)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        if !self.options.dry_run {
            let id = request.send().await?;
            if is_terminal!(f) {
                verbose!(f, "Created bug {id}")?;
            } else {
                writeln!(f, "{id}")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
