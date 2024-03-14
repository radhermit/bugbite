use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use bugbite::service::redmine::IssueField;
use bugbite::time::TimeDelta;
use bugbite::traits::WebClient;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::VariantNames;

use crate::macros::async_block;
use crate::service::output::render_search;

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

    /// restrict by ID
    #[arg(long, help_heading = "Attribute related")]
    ids: Option<Vec<MaybeStdinVec<u64>>>,

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

    /// string to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut query = client.service().search_query();
        let params = &self.params;
        if let Some(values) = params.ids.as_ref() {
            query.ids(values.iter().flatten());
        }
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
        render_search(issues, fields)?;

        Ok(ExitCode::SUCCESS)
    }
}
