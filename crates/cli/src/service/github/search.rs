use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::query::Order;
use bugbite::service::github::search::{OrderField, Parameters};
use bugbite::service::github::Service;
use bugbite::traits::RequestSend;
use clap::Args;
use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use crate::utils::COLUMNS;

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
        value_name = "TERM"
    )]
    order: Option<Order<OrderField>>,

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
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        let params: Parameters = self.params.into();

        let issues = service.search().params(params).send().await?;
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
