use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::objects::bugzilla::Flag;
use bugbite::output::verbose;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::service::bugzilla::create::*;
use bugbite::traits::{Merge, RequestSend, RequestTemplate};
use bugbite::utils::is_terminal;
use clap::Args;
use itertools::Itertools;

use crate::service::TemplateOptions;

#[derive(Args, Debug)]
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

            custom_fields: value
                .custom_fields
                .map(|x| x.into_iter().tuples().collect()),
        }
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Create options")]
pub(super) struct Options {
    /// read attributes from an existing bug
    #[arg(long, value_name = "ID", conflicts_with = "from")]
    from_bug: Option<String>,
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    template: TemplateOptions,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let mut request = service.create();

        // merge attributes from templates or bug
        if let Some(names) = &self.template.from {
            for name in names {
                request.load_template(name)?;
            }
        } else if let Some(id) = self.options.from_bug {
            let bug = service
                .get([id])
                .send()
                .await?
                .into_iter()
                .next()
                .expect("failed getting bug");
            request.params.merge(bug.into());
        }

        // command line parameters override template
        request.params.merge(self.params.into());

        // write attributes to template
        if let Some(name) = &self.template.to {
            request.save_template(name)?;
        }

        if !self.template.dry_run {
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
