use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::{
    search::{QueryBuilder, SearchOrder, SearchTerm},
    FilterField,
};
use bugbite::time::TimeDelta;
use clap::builder::BoolishValueParser;
use clap::Args;
use itertools::Itertools;
use strum::VariantNames;
use tracing::info;
use unicode_segmentation::UnicodeSegmentation;

use crate::macros::async_block;
use crate::utils::COLUMNS;

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Args)]
struct Params {
    /// fields to output
    #[arg(
        short = 'F',
        long,
        help_heading = "Search related",
        value_name = "FIELD[,FIELD,...]",
        value_delimiter = ','
    )]
    fields: Option<Vec<FilterField>>,

    /// sorting order for search query
    #[arg(
        short = 'S',
        long,
        help_heading = "Search related",
        value_name = "TERM[,TERM,...]",
        value_delimiter = ',',
        long_help = indoc::formatdoc! {"
            Perform server-side sorting on the given query.

            Sorting in descending order can be done by prefixing a given term
            with '-'; otherwise, sorting is performed in ascending order by
            default. Note that using a single descending order argument requires
            using '=' between the option and value such as `-S=-status` or
            `--sort=-summary`.

            Multiple terms are supported in a comma-separated list which will
            cause the data response to be sorted by the each term in order. For
            example, the value `reporter,-status` will sort by the bug reporter
            in ascending order and then by status in descending order.

            Note that if an invalid sorting request is made, bugzilla will fall
            back to its default which is sorting by bug ID. Also, some sorting
            methods such as last-visited require an authenticated session to
            work properly.

            possible values:
            {}", SearchTerm::VARIANTS.join(", ")}
    )]
    sort: Option<Vec<SearchOrder>>,

    /// search using quicksearch syntax
    #[arg(
        short = 'Q',
        long,
        help_heading = "Search related",
        long_help = indoc::indoc! {"
            Search for bugs using quicksearch syntax.

            For more information see:
            https://bugzilla.mozilla.org/page.cgi?id=quicksearch.html"}
    )]
    quicksearch: Option<String>,

    /// person the bug is assigned to
    #[arg(short, long, help_heading = "Person related")]
    assigned_to: Option<Vec<String>>,

    /// person who reported
    #[arg(short, long, help_heading = "Person related")]
    reporter: Option<Vec<String>>,

    /// person in the CC list
    #[arg(long, help_heading = "Person related")]
    cc: Option<Vec<String>>,

    /// person who commented
    #[arg(long, help_heading = "Person related")]
    commenter: Option<Vec<String>>,

    /// restrict by alias
    #[arg(long, help_heading = "Attribute related")]
    alias: Option<Vec<String>>,

    /// restrict by ID
    #[arg(long, help_heading = "Attribute related")]
    id: Option<Vec<MaybeStdinVec<u64>>>,

    /// restrict by component
    #[arg(short = 'C', long, help_heading = "Attribute related")]
    component: Option<String>,

    /// restrict by product
    #[arg(short = 'P', long, help_heading = "Attribute related")]
    product: Option<String>,

    /// restrict by URL
    #[arg(short = 'U', long, help_heading = "Attribute related")]
    url: Option<Vec<String>>,

    /// restrict by keyword
    #[arg(short = 'K', long, help_heading = "Attribute related")]
    keywords: Option<Vec<String>>,

    /// restrict by status
    #[arg(short, long, help_heading = "Attribute related")]
    status: Option<Vec<String>>,

    /// specified range of votes
    #[arg(long, help_heading = "Attribute related")]
    votes: Option<u32>,

    /// specified range of comments
    #[arg(long, help_heading = "Attribute related")]
    comments: Option<u32>,

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
    blocks: Option<Vec<MaybeStdinVec<u64>>>,

    /// restrict by dependencies
    #[arg(short = 'D', long, help_heading = "Attribute related")]
    depends: Option<Vec<MaybeStdinVec<u64>>>,

    /// created at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    created: Option<TimeDelta>,

    /// modified at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    modified: Option<TimeDelta>,

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
        if let Some(value) = params.created.as_ref() {
            query.created_after(value);
        }
        if let Some(value) = params.modified.as_ref() {
            query.modified_after(value);
        }
        if let Some(value) = params.sort.as_ref() {
            query.sort(value);
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
        if let Some(value) = params.fields.as_ref() {
            query.fields(value);
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
        if let Some(values) = params.alias.as_ref() {
            query.extend("alias", values);
        }
        if let Some(values) = params.id.as_ref() {
            query.extend("id", values.iter().flatten());
        }
        if let Some(values) = params.keywords.as_ref() {
            query.extend("keywords", values);
        }
        if let Some(values) = params.status.as_ref() {
            query.extend("status", values);
        }
        if let Some(values) = params.blocks.as_ref() {
            query.extend("blocks", values.iter().flatten());
        }
        if let Some(values) = params.depends.as_ref() {
            query.extend("depends_on", values.iter().flatten());
        }
        if let Some(values) = params.summary.as_ref() {
            query.extend("summary", values.iter().flatten());
        }

        let bugs = async_block!(client.search(query))?;
        let mut stdout = stdout().lock();
        let mut count = 0;

        for bug in bugs {
            count += 1;
            let line = bug.search_display();
            if line.len() > *COLUMNS {
                // truncate line to the terminal width of graphemes
                let mut iter = UnicodeSegmentation::graphemes(line.as_str(), true).take(*COLUMNS);
                writeln!(stdout, "{}", iter.join(""))?;
            } else {
                writeln!(stdout, "{line}")?;
            }
        }

        if count > 0 {
            info!(" * {count} found");
        }

        Ok(ExitCode::SUCCESS)
    }
}
