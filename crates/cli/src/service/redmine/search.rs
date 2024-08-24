use std::fs;
use std::io::stdout;
use std::process::ExitCode;

use anyhow::Context;
use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::objects::RangeOrValue;
use bugbite::query::Order;
use bugbite::service::redmine::search::{OrderField, Parameters};
use bugbite::service::redmine::IssueField;
use bugbite::service::redmine::Service;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{RequestMerge, RequestSend};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};

use crate::service::output::render_search;
use crate::utils::{confirm, launch_browser};

#[derive(Args)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(short, long, value_name = "FIELD[,...]", default_value = "id,subject")]
    fields: Csv<IssueField>,

    /// limit the number of results
    #[arg(short, long)]
    limit: Option<u64>,

    /// order query results
    #[arg(short, long, value_name = "FIELD[,...]")]
    order: Option<Csv<Order<OrderField>>>,
}

#[derive(Args)]
#[clap(next_help_heading = "Attribute options")]
struct AttributeOptions {
    /// restrict by assignee status
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
    )]
    assignee: Option<bool>,

    /// restrict by attachments
    #[arg(
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    attachments: Option<ExistsOrValues<String>>,

    /// restrict by blockers
    #[arg(
        short = 'B',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    blocks: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by dependencies
    #[arg(
        short = 'D',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    blocked: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by relations
    #[arg(
        short = 'R',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    relates: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by ID
    #[arg(long, value_name = "ID[,...]", value_delimiter = ',')]
    id: Option<Vec<MaybeStdinVec<RangeOrValue<u64>>>>,

    /// restrict by status
    #[arg(
        short,
        long,
        value_parser = ["open", "closed", "all"],
        hide_possible_values = true,
    )]
    status: Option<String>,
}

#[derive(Args)]
#[clap(next_help_heading = "Time options")]
struct TimeOptions {
    /// restrict by creation time
    #[arg(short, long, value_name = "TIME")]
    created: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by update time
    #[arg(short, long, value_name = "TIME")]
    updated: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by closed time
    #[arg(short = 'C', long, value_name = "TIME")]
    closed: Option<RangeOrValue<TimeDeltaOrStatic>>,
}

/// Available search parameters.
#[derive(Args)]
struct Params {
    #[clap(flatten)]
    query: QueryOptions,

    #[clap(flatten)]
    attr: AttributeOptions,

    #[clap(flatten)]
    time: TimeOptions,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<String>>>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            assignee: value.attr.assignee,
            attachments: value.attr.attachments,
            blocks: value.attr.blocks.map(|x| x.flatten()),
            blocked: value.attr.blocked.map(|x| x.flatten()),
            relates: value.attr.relates.map(|x| x.flatten()),
            ids: value.attr.id.map(|x| x.into_iter().flatten().collect()),
            created: value.time.created,
            updated: value.time.updated,
            closed: value.time.closed,
            status: value.attr.status,
            limit: value.query.limit,
            order: value.query.order.map(|x| x.into_iter().collect()),
            summary: value.summary.map(|x| x.into_iter().flatten().collect()),
        }
    }
}

#[derive(Args)]
#[clap(next_help_heading = "Search options")]
pub(super) struct Options {
    /// open in browser
    #[arg(short, long)]
    browser: bool,

    /// skip service interaction
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// output in JSON format
    #[arg(long)]
    json: bool,

    /// read attributes from template
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    from: Option<Utf8PathBuf>,

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
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        let mut request = service.search();

        // read attributes from template
        if let Some(path) = self.options.from.as_deref() {
            request.merge(path)?;
        }

        // command line parameters override template
        let fields = self.params.query.fields.clone();
        request.merge(self.params)?;

        // write attributes to template
        if let Some(path) = self.options.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&request)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        if self.options.browser {
            let url = request.search_url()?;
            launch_browser([url])?;
        } else if !self.options.dry_run {
            let items = request.send().await?;
            let stdout = stdout().lock();
            render_search(stdout, items, &fields, self.options.json)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
