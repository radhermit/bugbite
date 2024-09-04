use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::Csv;
use bugbite::query::Order;
use bugbite::service::github::search::{OrderField, Parameters};
use bugbite::service::github::Service;
use bugbite::traits::{Merge, RequestSend};
use bugbite::utils::is_terminal;
use clap::Args;
use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use crate::utils::COLUMNS;

/// Available search parameters.
#[derive(Args)]
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

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Service, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let mut request = service.search();
        request.params.merge(self.params.into());
        let issues = request.send().await?;
        let mut count = 0;

        for issue in issues {
            count += 1;
            let line = issue.search_display();
            if line.len() > *COLUMNS {
                // truncate line to the terminal width of graphemes
                let mut iter = UnicodeSegmentation::graphemes(line.as_str(), true).take(*COLUMNS);
                writeln!(f, "{}", iter.join(""))?;
            } else {
                writeln!(f, "{line}")?;
            }
        }

        if count > 0 && is_terminal!(f) {
            writeln!(f, " * {count} found")?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
