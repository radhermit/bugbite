use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::objects::RangeOrValue;
use bugbite::output::render_search;
use bugbite::query::Order;
use bugbite::service::redmine::search::{OrderField, Parameters};
use bugbite::service::redmine::IssueField;
use bugbite::service::redmine::Redmine;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{Merge, RequestStream, RequestTemplate};
use clap::Args;

use crate::service::TemplateOptions;
use crate::utils::launch_browser;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(short, long, value_name = "FIELD[,...]", default_value = "id,subject")]
    fields: Csv<IssueField>,

    /// limit result count
    #[arg(short, long)]
    limit: Option<usize>,

    /// result starting position
    #[arg(short = 'O', long)]
    offset: Option<usize>,

    /// order query results
    #[arg(short, long, value_name = "FIELD[,...]")]
    order: Option<Csv<Order<OrderField>>>,

    /// enable paging support
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
    )]
    paged: Option<bool>,
}

#[derive(Args, Debug)]
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
        short = '@',
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
        value_parser = ["@open", "@closed", "@any"],
        hide_possible_values = true,
    )]
    status: Option<String>,
}

#[derive(Args, Debug)]
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
#[derive(Args, Debug)]
struct Params {
    #[clap(flatten)]
    query: QueryOptions,

    #[clap(flatten)]
    attr: AttributeOptions,

    #[clap(flatten)]
    time: TimeOptions,

    /// restrict by subject content
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    subject: Option<Vec<MaybeStdinVec<String>>>,
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
            offset: value.query.offset,
            order: value.query.order.map(|x| x.into_iter().collect()),
            paged: value.query.paged,
            subject: value.subject.map(|x| x.into_iter().flatten().collect()),
        }
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Search options")]
pub(super) struct Options {
    /// open in browser
    #[arg(short, long)]
    browser: bool,

    /// output in JSON format
    #[arg(long)]
    json: bool,
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
    pub(super) async fn run<W>(self, service: &Redmine, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let mut request = service.search();

        // read attributes from templates
        if let Some(names) = &self.template.from {
            for name in names {
                request.load_template(name)?;
            }
        }

        // command line parameters override template
        let fields = self.params.query.fields.clone();
        request.params.merge(self.params.into());

        // write attributes to template
        if let Some(name) = self.template.to.as_ref() {
            request.save_template(name)?;
        }

        if self.options.browser {
            let url = request.search_url()?;
            launch_browser([url])?;
        } else if !self.template.dry_run {
            let items = request.stream();
            render_search(f, items, &fields, self.options.json).await?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
