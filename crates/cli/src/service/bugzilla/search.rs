use std::collections::HashMap;
use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::{
    search::{QueryBuilder, SearchOrder, SearchTerm},
    BugField,
};
use bugbite::time::TimeDelta;
use clap::builder::BoolishValueParser;
use clap::Args;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use strum::VariantNames;
use unicode_segmentation::UnicodeSegmentation;

use crate::macros::async_block;
use crate::utils::COLUMNS;

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[skip_serializing_none]
#[derive(Debug, Deserialize, Serialize, Args)]
struct Params {
    // TODO: use enum to define supported fields
    /// fields to output
    #[arg(short = 'F', long, help_heading = "Search related")]
    fields: Option<Csv<BugField>>,

    /// sorting order for search query
    #[arg(
        short = 'S',
        long,
        help_heading = "Search related",
        value_name = "TERM[,TERM,...]",
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
    sort: Option<Csv<SearchOrder>>,

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
    #[serde(rename = "creator")]
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
    id: Option<Vec<String>>,

    /// restrict by component
    #[arg(short = 'C', long, help_heading = "Attribute related")]
    component: Option<String>,

    /// restrict by product
    #[arg(short = 'P', long, help_heading = "Attribute related")]
    product: Option<String>,

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

    /// created at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    created: Option<TimeDelta>,

    /// modified at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    modified: Option<TimeDelta>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Box<Params>,
}

impl Command {
    pub(super) fn run(mut self, client: Client) -> anyhow::Result<ExitCode> {
        let mut query = QueryBuilder::new();

        if let Some(value) = self.params.created.take() {
            query.created_after(&value)?;
        }

        if let Some(value) = self.params.modified.take() {
            query.modified_after(&value)?;
        }

        if let Some(value) = self.params.sort.take() {
            query.sort(value);
        }

        if let Some(value) = self.params.commenter.take() {
            query.commenter(value)?;
        }

        if let Some(value) = self.params.votes.take() {
            query.votes(value);
        }

        if let Some(value) = self.params.comments.take() {
            query.comments(value);
        }

        if let Some(value) = self.params.attachments.take() {
            query.attachments(value);
        }

        if let Some(value) = self.params.fields.take() {
            query.fields(value)?;
        }

        // TODO: replace with a custom serde serializer to convert structs to parameter strings
        // convert search parameters to URL parameters
        let params = serde_json::to_string(&self.params).unwrap();
        let params: HashMap<String, Value> = serde_json::from_str(&params).unwrap();
        for (name, value) in params {
            match value {
                Value::String(value) => query.insert(name, value),
                Value::Array(values) => {
                    for value in values.iter().filter_map(|v| v.as_str()) {
                        query.append(&name, value);
                    }
                }
                value => panic!("invalid search parameter type: {value:?}"),
            }
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

        if count > 0 && stdout.is_terminal() {
            writeln!(stdout, " * {count} found")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
