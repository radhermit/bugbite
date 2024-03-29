use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::redmine::Client;
use bugbite::objects::RangeOrValue;
use bugbite::query::Order;
use bugbite::service::redmine::search::{ExistsField, OrderField};
use bugbite::service::redmine::IssueField;
use bugbite::time::TimeDelta;
use bugbite::traits::WebClient;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::Args;
use strum::VariantNames;

use crate::service::args::ExistsOrArray;
use crate::service::output::render_search;
use crate::utils::{launch_browser, wrapped_doc};

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
        default_value = "id,subject",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(IssueField::VARIANTS)
                .map(|s| s.parse::<IssueField>().unwrap()),
        long_help = wrapped_doc!("
            Restrict the data fields returned by the query.

            By default, only the id and subject fields are returned. This can be
            altered by specifying a custom list of fields which will change the
            output format to a space separated list of the field values for each
            item.

            Possible values: {}",
            IssueField::VARIANTS.join(", ")
        )
    )]
    fields: Vec<IssueField>,

    /// limit the number of results
    #[arg(
        short,
        long,
        help_heading = "Search options",
        long_help = wrapped_doc!("
            Limit the number of results.

            If the value is higher than the maximum limit that value is used
            instead and if the limit is set to zero, the default limit is used.
            Note that the maximum limit and default limit are generally not
            equal, most instances default to 100 and 25, respectively.
        ")
    )]
    limit: Option<u64>,

    /// order query results
    #[arg(
        short,
        long,
        value_name = "FIELD[,...]",
        value_delimiter = ',',
        help_heading = "Search options",
        long_help = wrapped_doc!("
            Perform server-side sorting on the query.

            Fields can be prefixed with `-` or `+` to sort in descending or
            ascending order, respectively. Unprefixed fields will use ascending
            order.

            Multiple fields are supported via comma-separated lists which sort
            the data response by the each field in order.

            Note that if an invalid sorting request is made, sorting will
            fallback to the service default.

            Possible values: {}",
            OrderField::VARIANTS.join(", ")
        )
    )]
    order: Option<Vec<Order<OrderField>>>,

    /// restrict by assignee status
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
        help_heading = "Attribute options",
        long_help = wrapped_doc!("
            Restrict by assignee status.

            With no argument, all matches with assignees are returned. If the
            value is `true` or `false`, all matches with or without assignees
            are returned, respectively.

            Examples:
            - assigned
            > bite s --assignee

            - unassigned
            > bite s --assignee false
        ")
    )]
    assignee: Option<bool>,

    /// restrict by attachments
    #[arg(
        long,
        num_args = 0..=1,
        help_heading = "Attribute options",
        value_name = "VALUE[,...]",
        default_missing_value = "true",
        long_help = wrapped_doc!(r#"
            Restrict query by attachments.

            With no argument, all matches with attachments are returned. If the
            value is `true` or `false`, all matches with or without attachments
            are returned, respectively.

            Examples:
            - existence
            > bite s --attachments

            - nonexistence
            > bite s --attachments false

            Regular string values search for matching substrings in an
            attachment's file name.

            Example:
            - contains `value`
            > bite s --attachments value

            Multiple values can be specified in a comma-separated list and will
            match if all of the specified values match.

            Example:
            - equals `test1` and `test2`
            > bite s --attachments test1,test2
        "#)
    )]
    attachments: Option<ExistsOrArray<String>>,

    /// restrict by blockers
    #[arg(
        short = 'B',
        long,
        num_args = 0..=1,
        help_heading = "Attribute options",
        value_name = "ID[,...]",
        default_missing_value = "true",
        long_help = wrapped_doc!("
            Restrict by blockers.

            With no argument, all matches with blockers are returned. If the
            value is `true` or `false`, all matches with or without blockers are
            returned, respectively.

            Examples:
            - existence
            > bite s --blocks

            - nonexistence
            > bite s --blocks false

            Regular values search for matching blockers and multiple values can
            be specified in a comma-separated list, matching if any of the
            specified blockers match.

            Examples:
            - blocked on 10
            > bite s --blocks 10

            - blocked on 10 and 11
            > bite s --blocks 10,11

            Values are taken from standard input when `-`.
        ")
    )]
    blocks: Option<ExistsOrArray<MaybeStdinVec<u64>>>,

    /// restrict by dependencies
    #[arg(
        short = 'D',
        long,
        num_args = 0..=1,
        help_heading = "Attribute options",
        value_name = "ID[,...]",
        default_missing_value = "true",
        long_help = wrapped_doc!("
            Restrict by dependencies.

            With no argument, all matches with dependencies are returned. If the
            value is `true` or `false`, all matches with or without dependencies
            are returned, respectively.

            Examples:
            - existence
            > bite s --blocked

            - nonexistence
            > bite s --blocked false

            Regular values search for matching dependencies and multiple values can
            be specified in a comma-separated list, matching if any of the
            specified dependencies match.

            Examples:
            - blocked on 10
            > bite s --blocked 10

            - blocked on 10 and 11
            > bite s --blocked 10,11

            Values are taken from standard input when `-`.
        ")
    )]
    blocked: Option<ExistsOrArray<MaybeStdinVec<u64>>>,

    /// restrict by relations
    #[arg(
        short = 'R',
        long,
        num_args = 0..=1,
        help_heading = "Attribute options",
        value_name = "ID[,...]",
        default_missing_value = "true",
        long_help = wrapped_doc!("
            Restrict by related issues.

            With no argument, all matches with relations are returned. If the
            value is `true` or `false`, all matches with or without relations
            are returned, respectively.

            Examples:
            - existence
            > bite s --relates

            - nonexistence
            > bite s --relates false

            Regular values search for matching relations and multiple values can
            be specified in a comma-separated list, matching if any of the
            specified relations match.

            Examples:
            - relates to 10
            > bite s --relates 10

            - relates to 10 and 11
            > bite s --relates 10,11

            Values are taken from standard input when `-`.
        ")
    )]
    relates: Option<ExistsOrArray<MaybeStdinVec<u64>>>,

    /// restrict by ID
    #[arg(
        long,
        help_heading = "Attribute options",
        value_name = "ID[,...]",
        value_delimiter = ','
    )]
    id: Option<Vec<MaybeStdinVec<u64>>>,

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
    created: Option<RangeOrValue<TimeDelta>>,

    /// restrict by modification time
    #[arg(short, long, value_name = "TIME", help_heading = "Time options")]
    modified: Option<RangeOrValue<TimeDelta>>,

    /// restrict by closed time
    #[arg(short = 'C', long, value_name = "TIME", help_heading = "Time options")]
    closed: Option<RangeOrValue<TimeDelta>>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<String>>>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    /// open query in a browser
    #[arg(
        short,
        long,
        help_heading = "Search options",
        long_help = wrapped_doc!("
            Open query in a browser.

            This functionality requires xdg-open with a valid, preferred browser
            set for http(s) URLs.
        ")
    )]
    browser: bool,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut query = client.service().search_query();
        let params = self.params;
        if let Some(value) = params.assignee {
            query.assignee(value);
        }
        if let Some(values) = params.attachments {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Attachment, value),
                ExistsOrArray::Array(values) => query.attachments(values.into_iter()),
            }
        }
        if let Some(values) = params.blocks {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Blocks, value),
                ExistsOrArray::Array(values) => query.blocks(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.blocked {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Blocked, value),
                ExistsOrArray::Array(values) => query.blocked(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.relates {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Relates, value),
                ExistsOrArray::Array(values) => query.relates(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.id.as_ref() {
            query.id(values.iter().flatten());
        }
        if let Some(value) = params.limit {
            query.limit(value);
        }
        if let Some(values) = params.order {
            query.order(values)?;
        }
        if let Some(value) = params.status.as_ref() {
            query.status(value)?;
        }
        if let Some(value) = params.closed.as_ref() {
            query.closed(value);
        }
        if let Some(value) = params.created.as_ref() {
            query.created(value);
        }
        if let Some(value) = params.modified.as_ref() {
            query.modified(value);
        }
        if let Some(values) = params.summary.as_ref() {
            query.summary(values.iter().flatten());
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

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_examples(&["redmine", "search"]);
    }
}
