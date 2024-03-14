use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::github::Client;
use bugbite::service::github::search::{SearchOrder, SearchTerm};
use bugbite::traits::WebClient;
use clap::Args;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::VariantNames;
use unicode_segmentation::UnicodeSegmentation;

use crate::macros::async_block;
use crate::utils::COLUMNS;

/// Available search parameters.
#[skip_serializing_none]
#[derive(Debug, Deserialize, Serialize, Args)]
struct Params {
    // TODO: use enum to define supported fields
    /// fields to output
    #[arg(short = 'F', long, help_heading = "Search related")]
    fields: Option<Csv<String>>,

    /// sorting order for search query
    #[arg(
        short = 'S',
        long,
        help_heading = "Search related",
        value_name = "TERM",
        long_help = indoc::formatdoc! {"
            Perform server-side sorting on the given query.

            Sorting in descending order can be done by prefixing a given term
            with '-'; otherwise, sorting is performed in ascending order by
            default. Note that using a single descending order argument requires
            using '=' between the option and value such as `-S=-created` or
            `--sort=-comments`.

            possible values:
            {}", SearchTerm::VARIANTS.join(", ")}
    )]
    sort: Option<SearchOrder>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) fn run(mut self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut query = client.service().search_query();

        if let Some(value) = self.params.sort.take() {
            query.sort(value);
        }

        let issues = async_block!(client.search(query))?;
        let mut stdout = stdout().lock();
        let mut count = 0;

        for issue in issues {
            count += 1;
            let line = issue.search_display();
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
