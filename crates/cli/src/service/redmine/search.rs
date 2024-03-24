use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use bugbite::objects::RangeOrEqual;
use bugbite::service::redmine::IssueField;
use bugbite::time::TimeDeltaIso8601;
use bugbite::traits::WebClient;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::VariantNames;

use crate::service::output::render_search;
use crate::utils::launch_browser;

/// Available search parameters.
#[derive(Debug, Args)]
struct Params {
    /// fields to output
    #[arg(
        short,
        long,
        help_heading = "Search options",
        value_name = "FIELD[,...]",
        value_delimiter = ',',
        default_value = "id,summary",
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

    /// limit the number of issues returned
    #[arg(
        short,
        long,
        help_heading = "Search options",
        long_help = indoc::formatdoc! {"
            Limit the number of issues returned.

            If the value is higher than the maximum limit that value is used
            instead and if the limit is set to zero, the default limit is used.
            Note that the maximum limit and default limit are generally not
            equal, most instances default to 100 and 25, respectively.
        "}
    )]
    limit: Option<u64>,

    /// restrict by ID
    #[arg(long, help_heading = "Attribute options")]
    ids: Option<Vec<MaybeStdinVec<u64>>>,

    /// restrict by status
    #[arg(
        short,
        long,
        help_heading = "Attribute options",
        value_parser = ["open", "closed", "all"],
    )]
    status: Option<String>,

    /// restrict by creation time
    #[arg(short, long, value_name = "TIME", help_heading = "Time options")]
    created: Option<RangeOrEqual<TimeDeltaIso8601>>,

    /// restrict by modification time
    #[arg(short, long, value_name = "TIME", help_heading = "Time options")]
    modified: Option<RangeOrEqual<TimeDeltaIso8601>>,

    /// string to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    /// open query in a browser
    #[arg(
        short,
        long,
        help_heading = "Search options",
        long_help = indoc::indoc! {"
            Open query in a browser.

            This functionality requires xdg-open with a valid, preferred browser
            set for http(s) URLs.
        "}
    )]
    browser: bool,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut query = client.service().search_query();
        let params = &self.params;
        if let Some(values) = params.ids.as_ref() {
            query.ids(values.iter().flatten());
        }
        if let Some(value) = params.limit {
            query.limit(value);
        }
        if let Some(value) = params.status.as_ref() {
            query.status(value)?;
        }
        if let Some(value) = params.created.as_ref() {
            query.created(value);
        }
        if let Some(value) = params.modified.as_ref() {
            query.modified(value);
        }
        if let Some(value) = params.summary.as_ref() {
            query.summary(value);
        }
        let fields = &params.fields;

        if self.browser {
            let url = client.search_url(query)?;
            launch_browser([url])?;
        } else {
            let issues = client.search(query).await?;
            render_search(issues, fields)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
