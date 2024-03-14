use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::{
    search::{QueryBuilder, SearchOrder, SearchTerm},
    BugField,
};
use bugbite::time::TimeDelta;
use clap::builder::{BoolishValueParser, PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::VariantNames;

use crate::macros::async_block;
use crate::service::output::render_search;
use crate::utils::launch_browser;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attribute options")]
struct AttributeOptions {
    /// restrict by attachment status
    #[arg(
        short = 'A',
        long,
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        hide_possible_values = true,
    )]
    attachments: Option<bool>,

    /// restrict by blockers
    #[arg(short = 'B', long, num_args = 0..=1, value_delimiter = ',')]
    blocks: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// specified range of comments
    #[arg(long)]
    comments: Option<u32>,

    /// restrict by component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// restrict by dependencies
    #[arg(short = 'D', long, num_args = 0..=1, value_delimiter = ',')]
    depends_on: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// restrict by group
    #[arg(short = 'G', long, num_args = 0..=1, value_delimiter = ',')]
    groups: Option<Vec<String>>,

    /// restrict by ID
    #[arg(long)]
    id: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// restrict by keyword
    #[arg(short = 'K', long, num_args = 0..=1, value_delimiter = ',')]
    keywords: Option<Vec<String>>,

    /// restrict by OS
    #[arg(long)]
    os: Option<String>,

    /// restrict by platform
    #[arg(long)]
    platform: Option<String>,

    /// restrict by product
    #[arg(short = 'P', long)]
    product: Option<String>,

    /// restrict by resolution
    #[arg(short = 'R', long)]
    resolution: Option<Vec<String>>,

    /// restrict by status
    #[arg(short, long)]
    status: Option<Vec<String>>,

    /// restrict by URL
    #[arg(short = 'U', long)]
    url: Option<Vec<String>>,

    /// restrict by version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// specified range of votes
    #[arg(long)]
    votes: Option<u32>,

    /// restrict by whiteboard
    #[arg(short = 'W', long)]
    whiteboard: Option<String>,
}

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Args)]
struct Params {
    /// open bugs in a browser
    #[arg(
        short,
        long,
        help_heading = "Search options",
        long_help = indoc::indoc! {"
            Open bugs in a browser.

            This functionality requires xdg-open with a valid, preferred browser
            set for http(s) URLs.
        "}
    )]
    browser: bool,

    /// fields to output
    #[arg(
        short,
        long,
        help_heading = "Search options",
        value_name = "FIELD[,FIELD,...]",
        value_delimiter = ',',
        default_value = "id,summary",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(BugField::VARIANTS)
                .map(|s| s.parse::<BugField>().unwrap()),
        long_help = indoc::formatdoc! {"
            Restrict the data fields returned by the query.

            By default, only the id, assignee, and summary fields of a bug are
            returned. This can be altered by specifying a custom list of fields
            instead which will also change the output format to a space
            separated list of the field values for each bug.

            possible values:
            {}", BugField::VARIANTS.join(", ")}
    )]
    fields: Vec<BugField>,

    /// limit the number of bugs returned
    #[arg(short, long, help_heading = "Search options")]
    limit: Option<NonZeroU64>,

    /// order query results
    #[arg(
        short,
        long,
        help_heading = "Search options",
        value_name = "FIELD[,FIELD,...]",
        value_delimiter = ',',
        long_help = indoc::formatdoc! {"
            Perform server-side sorting on the query.

            Sorting in descending order can be done by prefixing a given field
            with '-'; otherwise, sorting is performed in ascending order by
            default. Note that using a single descending order argument requires
            using '=' between the option and value such as `-S=-status` or
            `--sort=-summary`.

            Multiple fields are supported via comma-separated lists which sort
            the data response by the each field in order. For example, the value
            `reporter,-status` will sort by the bug reporter in ascending order
            and then by status in descending order.

            Note that if an invalid sorting request is made, sorting will
            fallback to bug ID. Also, some sorting methods such as last-visited
            require an authenticated session to work properly.

            possible values:
            {}", SearchTerm::VARIANTS.join(", ")}
    )]
    order: Option<Vec<SearchOrder>>,

    /// search using query grammar
    #[arg(short = 'Q', long, help_heading = "Search options")]
    query: Option<String>,

    /// search using quicksearch syntax
    #[arg(
        short = 'S',
        long,
        help_heading = "Search options",
        long_help = indoc::indoc! {"
            Search for bugs using quicksearch syntax.

            For more information see:
            https://bugzilla.mozilla.org/page.cgi?id=quicksearch.html
        "}
    )]
    quicksearch: Option<String>,

    /// user the bug is assigned to
    #[arg(
        short,
        long,
        help_heading = "User options",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    assigned_to: Option<Vec<String>>,

    /// user in the CC list
    #[arg(
        long,
        help_heading = "User options",
        value_name = "USER[,USER,...]",
        num_args = 0..=1,
        value_delimiter = ','
    )]
    cc: Option<Vec<String>>,

    /// user who commented
    #[arg(
        long,
        help_heading = "User options",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    commenters: Option<Vec<String>>,

    /// user who reported
    #[arg(
        short,
        long,
        help_heading = "User options",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    reporter: Option<Vec<String>>,

    #[clap(flatten)]
    attr: AttributeOptions,

    /// created at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time options")]
    created: Option<TimeDelta>,

    /// modified at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time options")]
    modified: Option<TimeDelta>,

    /// strings to search for in comments
    #[clap(long, value_name = "TERM", help_heading = "Content options")]
    comment: Option<Vec<MaybeStdinVec<String>>>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<String>>>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Box<Params>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        // TODO: implement a custom serde serializer to convert structs to URL parameters
        let mut query = QueryBuilder::new();
        let params = &self.params;

        // custom
        if let Some(value) = params.limit.as_ref() {
            query.limit(*value);
        }
        if let Some(value) = params.created.as_ref() {
            query.created_after(value);
        }
        if let Some(value) = params.modified.as_ref() {
            query.modified_after(value);
        }
        if let Some(value) = params.order.as_ref() {
            query.order(value);
        }
        if let Some(values) = params.commenters.as_ref() {
            query.commenters(values);
        }
        if let Some(values) = params.attr.url.as_ref() {
            query.url(values);
        }
        if let Some(value) = params.attr.votes {
            query.votes(value);
        }
        if let Some(value) = params.attr.comments {
            query.comments(value);
        }
        if let Some(value) = params.attr.attachments {
            query.attachments(value);
        }
        if let Some(values) = params.attr.id.as_ref() {
            query.id(values.iter().flatten().copied());
        }
        if let Some(values) = params.comment.as_ref() {
            query.comment(values.iter().flatten());
        }
        if let Some(values) = params.summary.as_ref() {
            query.summary(values.iter().flatten());
        }
        if let Some(values) = params.attr.groups.as_ref() {
            query.groups(values);
        }
        if let Some(values) = params.attr.keywords.as_ref() {
            query.keywords(values);
        }
        if let Some(values) = params.cc.as_ref() {
            query.cc(values);
        }
        if let Some(values) = params.attr.blocks.as_ref() {
            let values: Vec<_> = values.iter().flatten().copied().collect();
            query.blocks(&values);
        }
        if let Some(values) = params.attr.depends_on.as_ref() {
            let values: Vec<_> = values.iter().flatten().copied().collect();
            query.depends_on(&values);
        }

        // strings
        if let Some(value) = params.quicksearch.as_ref() {
            query.insert("quicksearch", value);
        }
        if let Some(value) = params.attr.component.as_ref() {
            query.insert("component", value);
        }
        if let Some(value) = params.attr.product.as_ref() {
            query.insert("product", value);
        }
        if let Some(value) = params.attr.version.as_ref() {
            query.insert("version", value);
        }
        if let Some(value) = params.attr.platform.as_ref() {
            query.insert("platform", value);
        }
        if let Some(value) = params.attr.os.as_ref() {
            query.insert("op_sys", value);
        }
        if let Some(value) = params.attr.whiteboard.as_ref() {
            query.insert("whiteboard", value);
        }

        // vectors
        if let Some(values) = params.assigned_to.as_ref() {
            query.extend("assigned_to", values);
        }
        if let Some(values) = params.reporter.as_ref() {
            query.extend("creator", values);
        }
        if let Some(values) = params.attr.status.as_ref() {
            query.extend("status", values);
        }
        if let Some(values) = params.attr.resolution.as_ref() {
            query.extend("resolution", values);
        }

        let fields = &params.fields;
        query.fields(fields.iter().copied())?;

        let bugs = async_block!(client.search(query))?;

        if params.browser {
            let urls = bugs.iter().map(|b| client.item_url(b.id));
            launch_browser(urls)?;
        } else {
            render_search(bugs, fields)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
