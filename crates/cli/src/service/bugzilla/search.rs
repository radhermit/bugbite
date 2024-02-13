use std::collections::HashMap;
use std::io::{stdout, IsTerminal};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::Client;
use bugbite::service::bugzilla::{QueryBuilder, SearchOrder, SearchTerm};
use bugbite::time::TimeDelta;
use bugbite::traits::WebService;
use clap::Args;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use strum::VariantNames;
use tokio::runtime::Handle;
use tokio::task;
use unicode_segmentation::UnicodeSegmentation;

use crate::utils::{launch_browser, COLUMNS};

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[skip_serializing_none]
#[derive(Debug, Deserialize, Serialize, Args)]
struct Params {
    // TODO: use enum to define supported fields
    /// fields to output
    #[arg(short, long, help_heading = "Search related")]
    fields: Option<Csv<String>>,

    /// sorting order for search query
    #[arg(
        long,
        help_heading = "Search related",
        value_name = "TERM[,TERM,...]",
        long_help = indoc::formatdoc! {"
            Requested sorting order for the given search query.

            Sorting in descending order can be done by prefixing a given term
            with '-'; otherwise, sorting is performed in ascending fashion by
            default.

            Multiple terms are supported in a comma-separated list which will
            cause the data response to be sorted by the each term in order.

            Note that if an invalid sorting request is made, bugzilla will fall
            back to its default which is sorting by bug ID. Also, some sorting
            methods such as last-visited require an authenticated session to
            work properly.

            Possible values:
              {}", SearchTerm::VARIANTS.iter().join("\n  ")}
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
    #[arg(long, value_name = "BOOL", help_heading = "Attribute related")]
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
    /// open query in a browser
    #[arg(short = 'B', long, help_heading = "Search related")]
    browser: bool,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) fn run(mut self, client: &Client) -> anyhow::Result<ExitCode> {
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

        if self.browser {
            let request = client.service().search_request(query)?;
            launch_browser([request.url().as_str()])?;
        } else {
            let bugs = task::block_in_place(move || {
                Handle::current().block_on(async { client.search(query).await })
            })?;

            let mut count = 0;
            for bug in bugs {
                count += 1;
                let line = bug.search_display();
                if line.len() > *COLUMNS {
                    // truncate line to the terminal width of graphemes
                    let mut iter =
                        UnicodeSegmentation::graphemes(line.as_str(), true).take(*COLUMNS);
                    println!("{}", iter.join(""));
                } else {
                    println!("{line}");
                }
            }

            if count > 0 && stdout().is_terminal() {
                println!(" * {count} found");
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
