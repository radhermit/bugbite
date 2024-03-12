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

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Args)]
struct Params {
    /// open bugs in browser
    #[arg(short, long)]
    browser: bool,

    /// fields to output
    #[arg(
        short,
        long,
        help_heading = "Search related",
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
    #[arg(short, long, help_heading = "Search related")]
    limit: Option<NonZeroU64>,

    /// order query results
    #[arg(
        short,
        long,
        help_heading = "Search related",
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
    #[arg(short = 'Q', long, help_heading = "Search related")]
    query: Option<String>,

    /// search using quicksearch syntax
    #[arg(
        short = 'S',
        long,
        help_heading = "Search related",
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
        help_heading = "User related",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    assigned_to: Option<Vec<String>>,

    /// user in the CC list
    #[arg(
        long,
        help_heading = "User related",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    cc: Option<Vec<String>>,

    /// user who commented
    #[arg(
        long,
        help_heading = "User related",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    commenter: Option<Vec<String>>,

    /// user who reported
    #[arg(
        short,
        long,
        help_heading = "User related",
        value_name = "USER[,USER,...]",
        value_delimiter = ','
    )]
    reporter: Option<Vec<String>>,

    /// restrict by attachment status
    #[arg(
        short = 'A',
        long,
        help_heading = "Attribute related",
        value_name = "BOOL",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
        hide_possible_values = true,
    )]
    attachments: Option<bool>,

    /// restrict by blockers
    #[arg(short = 'B', long, help_heading = "Attribute related")]
    blocks: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// specified range of comments
    #[arg(long, help_heading = "Attribute related")]
    comments: Option<u32>,

    /// restrict by component
    #[arg(short = 'C', long, help_heading = "Attribute related")]
    component: Option<String>,

    /// restrict by dependencies
    #[arg(short = 'D', long, help_heading = "Attribute related")]
    depends: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// restrict by ID
    #[arg(long, help_heading = "Attribute related")]
    id: Option<Vec<MaybeStdinVec<NonZeroU64>>>,

    /// restrict by keyword
    #[arg(short = 'K', long, help_heading = "Attribute related")]
    keywords: Option<Vec<String>>,

    /// restrict by OS
    #[arg(long, help_heading = "Attribute related")]
    os: Option<String>,

    /// restrict by platform
    #[arg(long, help_heading = "Attribute related")]
    platform: Option<String>,

    /// restrict by product
    #[arg(short = 'P', long, help_heading = "Attribute related")]
    product: Option<String>,

    /// restrict by resolution
    #[arg(short = 'R', long, help_heading = "Attribute related")]
    resolution: Option<Vec<String>>,

    /// restrict by status
    #[arg(short, long, help_heading = "Attribute related")]
    status: Option<Vec<String>>,

    /// restrict by URL
    #[arg(short = 'U', long, help_heading = "Attribute related")]
    url: Option<Vec<String>>,

    /// restrict by version
    #[arg(short = 'V', long, help_heading = "Attribute related")]
    version: Option<String>,

    /// specified range of votes
    #[arg(long, help_heading = "Attribute related")]
    votes: Option<u32>,

    /// restrict by whiteboard
    #[arg(short = 'W', long, help_heading = "Attribute related")]
    whiteboard: Option<String>,

    /// created at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    created: Option<TimeDelta>,

    /// modified at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    modified: Option<TimeDelta>,

    /// strings to search for in comments
    #[clap(long, value_name = "TERM", help_heading = "Content related")]
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
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
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
        if let Some(value) = params.commenter.as_ref() {
            query.commenter(value);
        }
        if let Some(values) = params.url.as_ref() {
            query.url(values);
        }
        if let Some(value) = params.votes {
            query.votes(value);
        }
        if let Some(value) = params.comments {
            query.comments(value);
        }
        if let Some(value) = params.attachments {
            query.attachments(value);
        }
        if let Some(values) = params.id.as_ref() {
            query.id(values.iter().flatten().copied());
        }
        if let Some(values) = params.comment.as_ref() {
            query.comment(values.iter().flatten());
        }
        if let Some(values) = params.summary.as_ref() {
            query.summary(values.iter().flatten());
        }

        // strings
        if let Some(value) = params.quicksearch.as_ref() {
            query.insert("quicksearch", value);
        }
        if let Some(value) = params.component.as_ref() {
            query.insert("component", value);
        }
        if let Some(value) = params.product.as_ref() {
            query.insert("product", value);
        }
        if let Some(value) = params.version.as_ref() {
            query.insert("version", value);
        }
        if let Some(value) = params.platform.as_ref() {
            query.insert("platform", value);
        }
        if let Some(value) = params.os.as_ref() {
            query.insert("op_sys", value);
        }
        if let Some(value) = params.whiteboard.as_ref() {
            query.insert("whiteboard", value);
        }

        // vectors
        if let Some(values) = params.assigned_to.as_ref() {
            query.extend("assigned_to", values);
        }
        if let Some(values) = params.reporter.as_ref() {
            query.extend("creator", values);
        }
        if let Some(values) = params.cc.as_ref() {
            query.extend("cc", values);
        }
        if let Some(values) = params.commenter.as_ref() {
            query.extend("commenter", values);
        }
        if let Some(values) = params.keywords.as_ref() {
            query.extend("keywords", values);
        }
        if let Some(values) = params.status.as_ref() {
            query.extend("status", values);
        }
        if let Some(values) = params.resolution.as_ref() {
            query.extend("resolution", values);
        }
        if let Some(values) = params.blocks.as_ref() {
            query.extend("blocks", values.iter().flatten());
        }
        if let Some(values) = params.depends.as_ref() {
            query.extend("depends_on", values.iter().flatten());
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
