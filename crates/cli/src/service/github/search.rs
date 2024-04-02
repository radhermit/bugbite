use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::client::github::Client;
use bugbite::service::github::search::{Parameters, SearchOrder, SearchTerm};
use clap::Args;
use itertools::Itertools;
use strum::VariantNames;
use unicode_segmentation::UnicodeSegmentation;

use crate::utils::{wrapped_doc, COLUMNS};

/// Available search parameters.
#[derive(Debug, Args)]
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
        long_help = wrapped_doc!("
            Perform server-side sorting on the given query.

            Sorting in descending order can be done by prefixing a given term
            with '-'; otherwise, sorting is performed in ascending order by
            default. Note that using a single descending order argument requires
            using '=' between the option and value such as `-S=-created` or
            `--sort=-comments`.

            Possible values: {}",
            SearchTerm::VARIANTS.join(", ")
        )
    )]
    order: Option<SearchOrder>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Vec<String>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self { order: value.order }
    }
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let params: Parameters = self.params.into();

        let issues = client.search(params).await?;
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
