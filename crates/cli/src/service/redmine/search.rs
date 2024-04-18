use std::fs;
use std::io::stdout;
use std::process::ExitCode;

use anyhow::Context;
use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::client::redmine::Client;
use bugbite::objects::RangeOrValue;
use bugbite::query::Order;
use bugbite::service::redmine::search::{OrderField, Parameters};
use bugbite::service::redmine::IssueField;
use bugbite::time::TimeDeltaOrStatic;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};

use crate::service::output::render_search;
use crate::utils::{confirm, launch_browser, wrapped_doc};

#[derive(Debug, Args)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(short, long, value_name = "FIELD[,...]", default_value = "id,subject")]
    fields: Csv<IssueField>,

    /// limit the number of results
    #[arg(short, long)]
    limit: Option<u64>,

    /// order query results
    #[arg(short, long, value_name = "FIELD[,...]")]
    order: Option<Csv<Order<OrderField>>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attribute options")]
struct AttributeOptions {
    /// restrict by assignee status
    #[arg(
        short,
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        hide_possible_values = true,
        value_name = "BOOL",
    )]
    assignee: Option<bool>,

    /// restrict by attachments
    #[arg(
        long,
        num_args = 0..=1,
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
    attachments: Option<ExistsOrValues<String>>,

    /// restrict by blockers
    #[arg(
        short = 'B',
        long,
        num_args = 0..=1,
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
    blocks: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by dependencies
    #[arg(
        short = 'D',
        long,
        num_args = 0..=1,
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
    blocked: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by relations
    #[arg(
        short = 'R',
        long,
        num_args = 0..=1,
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
    relates: Option<ExistsOrValues<MaybeStdinVec<u64>>>,

    /// restrict by ID
    #[arg(long, value_name = "ID[,...]", value_delimiter = ',')]
    id: Option<Vec<MaybeStdinVec<RangeOrValue<u64>>>>,

    /// restrict by status
    #[arg(
        short,
        long,
        value_parser = ["open", "closed", "all"],
    )]
    status: Option<String>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Time options")]
struct TimeOptions {
    /// restrict by creation time
    #[arg(short, long, value_name = "TIME")]
    created: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by modification time
    #[arg(short, long, value_name = "TIME")]
    modified: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by closed time
    #[arg(short = 'C', long, value_name = "TIME")]
    closed: Option<RangeOrValue<TimeDeltaOrStatic>>,
}

/// Available search parameters.
#[derive(Debug, Args)]
struct Params {
    #[clap(flatten)]
    query: QueryOptions,

    #[clap(flatten)]
    attr: AttributeOptions,

    #[clap(flatten)]
    time: TimeOptions,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<String>>>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            assignee: value.attr.assignee,
            attachments: value.attr.attachments,
            blocks: value.attr.blocks.map(|x| x.flatten()),
            blocked: value.attr.blocked.map(|x| x.flatten()),
            relates: value.attr.relates.map(|x| x.flatten()),
            ids: value.attr.id.map(|x| x.into_iter().flatten().collect()),
            created: value.time.created,
            modified: value.time.modified,
            closed: value.time.closed,
            status: value.attr.status,
            limit: value.query.limit,
            order: value.query.order.map(|x| x.into_iter().collect()),
            summary: value.summary.map(|x| x.into_iter().flatten().collect()),
        }
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Search options")]
pub(super) struct SearchOptions {
    /// open in browser
    #[arg(short, long)]
    browser: bool,

    /// skip service interaction
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// output in JSON format
    #[arg(long)]
    json: bool,

    /// read attributes from template
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to template
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
    )]
    to: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    search: SearchOptions,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let fields = self.params.query.fields.clone();
        let mut params: Parameters = self.params.into();

        // read attributes from template
        if let Some(path) = self.search.from.as_ref() {
            let template = Parameters::from_path(path)?;
            // command-line options override template options
            params = params.merge(template);
        }

        // write attributes to template
        if let Some(path) = self.search.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&params)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        if self.search.browser {
            let url = client.search_url(params)?;
            launch_browser([url])?;
        } else if !self.search.dry_run {
            let items = client.search(params).await?;
            let stdout = stdout().lock();
            render_search(stdout, items, &fields, self.search.json)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc(&["redmine", "search"]);
    }
}
