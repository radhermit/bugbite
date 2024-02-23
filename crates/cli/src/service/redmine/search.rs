use std::io::{stdout, Write};
use std::process::ExitCode;

use bugbite::client::redmine::Client;
use bugbite::service::redmine::search::QueryBuilder;
use bugbite::service::redmine::IssueField;
use bugbite::time::TimeDelta;
use bugbite::traits::RenderSearch;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::VariantNames;
use tracing::info;

use crate::macros::async_block;
use crate::utils::{truncate, COLUMNS};

/// Available search parameters.
#[derive(Debug, Args)]
struct Params {
    /// fields to output
    #[arg(
        short,
        long,
        help_heading = "Search related",
        value_name = "FIELD[,FIELD,...]",
        value_delimiter = ',',
        default_value = "id,assigned-to,summary",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(IssueField::VARIANTS)
                .map(|s| s.parse::<IssueField>().unwrap()),
        long_help = indoc::formatdoc! {"
            Restrict the data fields returned by the query.

            By default, only the id, assignee, and summary fields of a bug are
            returned. This can be altered by specifying a custom list of fields
            instead which will also change the output format to a space
            separated list of the field values for each bug.

            possible values:
            {}", IssueField::VARIANTS.join(", ")}
    )]
    fields: Vec<IssueField>,

    /// restrict by status
    #[arg(
        short,
        long,
        help_heading = "Attribute related",
        value_parser = ["open", "closed", "all"],
    )]
    status: Option<String>,

    /// created at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    created: Option<TimeDelta>,

    /// modified at this time or later
    #[arg(short, long, value_name = "TIME", help_heading = "Time related")]
    modified: Option<TimeDelta>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) fn run(&self, client: Client) -> anyhow::Result<ExitCode> {
        let mut query = QueryBuilder::new();
        let params = &self.params;
        if let Some(value) = params.status.as_ref() {
            query.status(value)?;
        }
        if let Some(value) = params.created.as_ref() {
            query.created_after(value);
        }
        if let Some(value) = params.modified.as_ref() {
            query.modified_after(value);
        }
        if let Some(value) = params.summary.as_ref() {
            query.summary(value);
        }
        let fields = &params.fields;

        let issues = async_block!(client.search(query))?;
        let mut stdout = stdout().lock();
        let mut count = 0;

        for issue in issues {
            count += 1;
            let line = issue.render(fields);
            writeln!(stdout, "{}", truncate(&line, *COLUMNS))?;
        }

        if count > 0 {
            info!(" * {count} found");
        }

        Ok(ExitCode::SUCCESS)
    }
}
