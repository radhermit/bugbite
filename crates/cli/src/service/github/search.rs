use std::io::{stdout, IsTerminal};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::Client;
use bugbite::service::github::{QueryBuilder, SearchOrder, SearchTerm};
use bugbite::traits::WebService;
use clap::Args;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum::VariantNames;
use tokio::runtime::Handle;
use tokio::task;
use unicode_segmentation::UnicodeSegmentation;

use crate::utils::{launch_browser, COLUMNS};

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
            Requested sorting order for the given search query.

            Sorting in descending order can be done by prefixing a given term
            with '-'; otherwise, sorting is performed in ascending fashion by
            default.

            Possible values:
              {}", SearchTerm::VARIANTS.iter().join("\n  ")}
    )]
    sort: Option<SearchOrder>,

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

        if let Some(value) = self.params.sort.take() {
            query.sort(value);
        }

        if self.browser {
            let request = client.service().search_request(query)?;
            launch_browser([request.url().as_str()])?;
        } else {
            let items = task::block_in_place(move || {
                Handle::current().block_on(async { client.search(query).await })
            })?;

            let mut count = 0;
            for item in items {
                count += 1;
                let line = item.search_display();
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
