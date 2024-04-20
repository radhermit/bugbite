use std::fs;
use std::io::stdout;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::Context;
use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::client::bugzilla::Client;
use bugbite::objects::RangeOrValue;
use bugbite::query::Order;
use bugbite::service::bugzilla::{
    search::{ChangeField, Match, OrderField, Parameters},
    BugField, FilterField, GroupField,
};
use bugbite::time::TimeDeltaOrStatic;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use crossterm::style::Stylize;
use itertools::Itertools;
use strum::VariantNames;

use crate::service::output::render_search;
use crate::utils::{confirm, launch_browser, wrapped_doc};

/// Parse a string into a ChangeField, adding possible values to the error on failure.
fn change_field(s: &str) -> anyhow::Result<ChangeField> {
    s.parse().map_err(|_| {
        let possible = ChangeField::VARIANTS.iter().map(|s| s.green()).join(", ");
        anyhow::anyhow!("invalid change field: {s}\n  [possible values: {possible}]")
    })
}

#[derive(Debug, Clone)]
struct Changed {
    fields: Vec<ChangeField>,
    interval: RangeOrValue<TimeDeltaOrStatic>,
}

impl FromStr for Changed {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((raw_fields, time)) = s.split_once('=') else {
            anyhow::bail!("missing time interval");
        };

        Ok(Self {
            fields: raw_fields.split(',').map(change_field).try_collect()?,
            interval: time.parse()?,
        })
    }
}

#[derive(Debug, Clone)]
struct ChangedBy {
    fields: Vec<ChangeField>,
    users: Vec<String>,
}

impl FromStr for ChangedBy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((raw_fields, users)) = s.split_once('=') else {
            anyhow::bail!("missing users");
        };

        Ok(Self {
            fields: raw_fields.split(',').map(change_field).try_collect()?,
            users: users.split(',').map(|s| s.to_string()).collect(),
        })
    }
}

#[derive(Debug, Clone)]
struct ChangedValue {
    field: ChangeField,
    value: String,
}

impl FromStr for ChangedValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((field, value)) = s.split_once('=') else {
            anyhow::bail!("missing value");
        };

        Ok(Self {
            field: change_field(field)?,
            value: value.to_string(),
        })
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Attribute options")]
struct AttributeOptions {
    /// restrict by alias
    #[arg(
        short = 'A',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
        long_help = wrapped_doc!(r#"
            Restrict query by an alias.

            With no argument, all bugs with aliases are returned. If the value
            is `true` or `false`, all bugs with or without aliases are returned,
            respectively.

            Examples:
            - existence
            > bite s --alias

            - nonexistence
            > bite s --alias false

            Regular values search for matching substrings and multiple values
            can be specified in a comma-separated list for logical AND or
            multiple options for logical OR.

            Examples:
            - contains `value`
            > bite s --alias value

            - contains `value1` and `value2`
            > bite s --alias value1,value1

            - contains `value1` or `value2`
            > bite s --alias value1 --alias value1

            Values can use match operator prefixes to alter their query
            application. Note that some operators may need to be escaped when
            used in the shell environment.

            Examples:
            - doesn't contain `value`
            > bite s --alias ~#value

            - equals `value`
            > bite s --alias =#value

            - doesn't equal `value`
            > bite s --alias ~=#value

            - matches regex
            > bite s --alias r#test?.+

            - doesn't match regex
            > bite s --alias ~r#test?.+
        "#)
    )]
    alias: Option<Vec<ExistsOrValues<Match>>>,

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
            attachment's description or file name.

            Example:
            - contains `value`
            > bite s --attachments value

            Values can use string matching prefixes to alter their application
            to queries. Note that some match operators may need to be escaped
            when used in the shell environment.

            Examples:
            - doesn't contain `value`
            > bite s --attachments ~#value

            - equals `value`
            > bite s --attachments =#value

            - doesn't equal `value`
            > bite s --attachments ~=#value

            - matches regex
            > bite s --attachments r#test?.+

            - doesn't match regex
            > bite s --attachments ~r#test?.+

            Multiple values can be specified in a comma-separated list and will
            match if any of the specified values match.

            Example:
            - equals `test1` or `test2`
            > bite s --attachments =#test1,=#test2
        "#)
    )]
    attachments: Option<ExistsOrValues<Match>>,

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
            be specified in a comma-separated list for logical AND or multiple
            options for logical OR.

            Examples:
            - blocked on 10
            > bite s --blocks 10

            - blocked on 10 and 11
            > bite s --blocks 10,11

            - blocked on 10 or 11
            > bite s --blocks 10 --blocks 11

            Values can use the `-` prefix to search for non-blockers.

            Examples:
            - isn't blocked on 10
            > bite s --blocks=-10

            - blocked on 10 but not 11
            > bite s --blocks 10,-11

            Values are taken from standard input when `-`.
        ")
    )]
    blocks: Option<Vec<ExistsOrValues<MaybeStdinVec<i64>>>>,

    /// restrict by component
    #[arg(short = 'C', long, value_name = "VALUE")]
    component: Option<Vec<Match>>,

    /// restrict by custom field
    #[arg(long = "cf", num_args = 2, value_names = ["NAME", "VALUE"])]
    custom_fields: Option<Vec<String>>,

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
            > bite s --depends

            - nonexistence
            > bite s --depends false

            Regular values search for matching dependencies and multiple values can
            be specified in a comma-separated list for logical AND or multiple
            options for logical OR.

            Examples:
            - depends on 10
            > bite s --depends 10

            - depends on 10 and 11
            > bite s --depends 10,11

            - depends on 10 or 11
            > bite s --depends 10 --depends 11

            Values can use the `-` prefix to search for non-dependencies.

            Examples:
            - doesn't depend on 10
            > bite s --depends=-10

            - depends on 10 but not 11
            > bite s --depends 10,-11

            Values are taken from standard input when `-`.
        ")
    )]
    depends: Option<Vec<ExistsOrValues<MaybeStdinVec<i64>>>>,

    /// restrict by flag
    #[arg(
        short = 'F',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    flags: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by group
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    groups: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by ID
    #[arg(
        long,
        num_args = 1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Restrict by ID.

            Regular values search for exact bug identifiers.

            Examples:
            - ID equal to 10
            > bite s --id 10

            Values can use the `-` prefix to search for identifiers not equal to
            the value.

            Examples:
            - all bugs except 10
            > bite s --id=-10

            Multiple values can be specified in a comma-separated list for
            logical OR.

            - IDs 10 or 20
            > bite s --id 10,20

            ID ranges are supported.

            - IDs greater than or equal to 10
            > bite s --id >=10

            - IDs between 10 and 20
            > bite s --id 10..20

            Values are taken from standard input when `-`.

            Example:
            - Any ID matching values taken from a file
            > cat file | bite s --id -
        ")
    )]
    id: Option<Vec<MaybeStdinVec<RangeOrValue<i64>>>>,

    /// restrict by keyword
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    keywords: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by OS
    #[arg(long, value_name = "VALUE")]
    os: Option<Vec<Match>>,

    /// restrict by platform
    #[arg(long, value_name = "VALUE")]
    platform: Option<Vec<Match>>,

    /// restrict by priority
    #[arg(long, value_name = "VALUE")]
    priority: Option<Vec<Match>>,

    /// restrict by product
    #[arg(short, long, value_name = "VALUE")]
    product: Option<Vec<Match>>,

    /// restrict by resolution
    #[arg(short, long, value_name = "VALUE")]
    resolution: Option<Vec<Match>>,

    /// restrict by external URLs
    #[arg(
        short = 'U',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    see_also: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by severity
    #[arg(long, value_name = "VALUE")]
    severity: Option<Vec<Match>>,

    /// restrict by status
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        num_args = 1,
        long_help = wrapped_doc!("
            Restrict by status.

            By default, searches with no status parameter only target open bugs.
            This option supports extending searches to closed bugs.

            Regular values search for case-insensitive matches. The prefix `~`
            can be used to search for all non-matches instead.

            Examples:
            - unconfirmed bugs
            > bite s --status unconfirmed

            - unresolved bugs
            > bite s --status ~resolved

            Multiple values can be specified in a comma-separated list for
            logical OR.

            Examples:
            - confirmed or verified bugs
            > bite s --status confirmed,verified

            The aliases `@open`, `@closed`, and `@all` can be used to search for
            open, closed, and all bugs, respectively.

            Examples:
            - closed bugs with bugbite in the summary
            > bite s --status @closed bugbite

            - all bugs assigned to yourself
            > bite s --status @all --assignee @me
        ")
    )]
    status: Option<Vec<String>>,

    /// restrict by personal tags
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    tags: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by target milestone
    #[arg(short = 'T', long, value_name = "VALUE")]
    target: Option<Vec<Match>>,

    /// restrict by URL
    #[arg(
        short = 'u',
        long,
        value_name = "VALUE[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    url: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by version
    #[arg(short = 'V', long, value_name = "VALUE")]
    version: Option<Vec<Match>>,

    /// restrict by whiteboard
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    whiteboard: Option<Vec<ExistsOrValues<Match>>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Range options")]
struct RangeOptions {
    /// restrict by comments
    #[arg(
        long,
        long_help = wrapped_doc!(r#"
            Restrict by the number of comments.

            Values can either be numbers, numbers prefixed with operators (e.g.
            <, ~=, >=), or ranges using the operators `..` and `..=`.

            Examples:
            - equal to 10
            > bite s --comments 10
            > bite s --comments "=10"

            - not equal to 10
            > bite s --comments "~=10"

            - greater than or equal to 10
            > bite s --comments ">=10"

            - between 5 and 10
            > bite s --comments 5..10
        "#)
    )]
    comments: Option<RangeOrValue<u64>>,

    /// restrict by votes
    #[arg(
        long,
        long_help = wrapped_doc!(r#"
            Restrict by the number of votes.

            Values can either be numbers, numbers prefixed with operators (e.g.
            <, ~=, >=), or ranges using the operators `..` and `..=`.

            Examples:
            - equal to 10
            > bite s --votes 10
            > bite s --votes "=10"

            - not equal to 10
            > bite s --votes "~=10"

            - greater than or equal to 10
            > bite s --votes ">=10"

            - between 5 and 10
            > bite s --votes 5..10
        "#)
    )]
    votes: Option<RangeOrValue<u64>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Change options")]
struct ChangeOptions {
    /// fields changed within time interval
    #[arg(
        long,
        value_name = "FIELD[,...]=TIME",
        long_help = wrapped_doc!("
            Restrict by fields changed within a time interval.

            Possible fields: {}",
            ChangeField::VARIANTS.join(", ")
        )
    )]
    changed: Option<Vec<Changed>>,

    /// fields changed by users
    #[arg(
        long,
        value_name = "FIELD[,...]=USER[,...]",
        long_help = wrapped_doc!("
            Restrict by fields changed by a given user.

            Possible fields: {}",
            ChangeField::VARIANTS.join(", ")
        )
    )]
    changed_by: Option<Vec<ChangedBy>>,

    /// fields changed from a value
    #[arg(
        long,
        value_name = "FIELD=VALUE",
        long_help = wrapped_doc!("
            Restrict by fields changed from a given value.

            Possible fields: {}",
            ChangeField::VARIANTS.join(", ")
        )
    )]
    changed_from: Option<Vec<ChangedValue>>,

    /// fields changed to a value
    #[arg(
        long,
        value_name = "FIELD=VALUE",
        long_help = wrapped_doc!("
            Restrict by fields changed to a given value.

            Possible fields: {}",
            ChangeField::VARIANTS.join(", ")
        )
    )]
    changed_to: Option<Vec<ChangedValue>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(
        short,
        long,
        value_name = "FIELD[,...]",
        default_value = "id,summary",
        long_help = wrapped_doc!("
            Restrict the data fields returned by the query.

            By default, only the id and summary fields are returned. This can be
            altered by specifying a list of fields which will only return and
            display the requested fields.

            Group fields relate to groups of bug fields and are most useful when
            using JSON formatted output:

            Example:
            - render all bugs created in the past day in JSON
            > bite s --created 1d -f all --json

            Possible group fields: {}
            Possible bug fields: {}",
            GroupField::VARIANTS.join(", "),
            BugField::VARIANTS.join(", ")
        )
    )]
    fields: Csv<FilterField>,

    /// limit the number of results
    #[arg(
        short,
        long,
        long_help = wrapped_doc!("
            Limit the number of results.

            If the value is higher than the maximum service limit that value is
            used instead. If the limit is set to zero, all matching results are
            returned.
        ")
    )]
    limit: Option<u64>,

    /// order query results
    #[arg(
        short,
        long,
        value_name = "FIELD[,...]",
        long_help = wrapped_doc!("
            Perform server-side sorting on the query.

            Fields can be prefixed with `-` or `+` to sort in descending or
            ascending order, respectively. Unprefixed fields will use ascending
            order.

            Multiple fields are supported via comma-separated lists which sort
            by each field in order.

            Note that if an invalid sorting request is made, sorting will
            fallback to the service default.

            Ordering is especially useful in combination with -l/--limit to get
            the first or last results of an ordered match.

            Examples:
            - top ten bugs by votes
            > bite s --limit 10 --order=-votes

            - highest comment count
            > bite s --limit 1 --order=-comments

            Possible values: {}",
            OrderField::VARIANTS.join(", ")
        )
    )]
    order: Option<Csv<Order<OrderField>>>,

    /// search using quicksearch syntax
    #[arg(
        short = 'S',
        long,
        value_name = "QUERY",
        long_help = wrapped_doc!("
            Search for bugs using quicksearch syntax.

            For more information see:
            https://bugzilla.mozilla.org/page.cgi?id=quicksearch.html
        ")
    )]
    quicksearch: Option<String>,
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
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "User options")]
struct UserOptions {
    /// user is the assignee
    #[arg(short, long, value_name = "USER[,...]")]
    assignee: Option<Vec<Csv<Match>>>,

    /// user created attachment
    #[arg(long, value_name = "USER[,...]")]
    attacher: Option<Vec<Csv<Match>>>,

    /// user in the CC list
    #[arg(
        long,
        value_name = "USER[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    cc: Option<Vec<ExistsOrValues<Match>>>,

    /// user who commented
    #[arg(long, value_name = "USER[,...]")]
    commenter: Option<Vec<Csv<Match>>>,

    /// user who set a flag
    #[arg(long, value_name = "USER[,...]")]
    flagger: Option<Vec<Csv<Match>>>,

    /// user is the QA contact
    #[arg(
        long,
        value_name = "USER[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    qa: Option<Vec<ExistsOrValues<Match>>>,

    /// user who reported
    #[arg(short = 'R', long, value_name = "USER[,...]")]
    reporter: Option<Vec<Csv<Match>>>,
}

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Debug, Args)]
struct Params {
    #[clap(flatten)]
    query: QueryOptions,

    #[clap(flatten)]
    attr: AttributeOptions,

    #[clap(flatten)]
    range: RangeOptions,

    #[clap(flatten)]
    change: ChangeOptions,

    #[clap(flatten)]
    time: TimeOptions,

    #[clap(flatten)]
    user: UserOptions,

    /// strings to search for in comments
    #[clap(long, value_name = "TERM", help_heading = "Content options")]
    comment: Option<Vec<MaybeStdinVec<Match>>>,

    /// strings to search for in the summary
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<Match>>>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            fields: Some(value.query.fields.into_iter().map(Into::into).collect()),
            limit: value.query.limit,
            order: value.query.order.map(|x| x.into_iter().collect()),
            quicksearch: value.query.quicksearch,

            alias: value.attr.alias,
            attachments: value.attr.attachments,
            flags: value.attr.flags,
            groups: value.attr.groups,
            keywords: value.attr.keywords,
            see_also: value.attr.see_also,
            tags: value.attr.tags,
            whiteboard: value.attr.whiteboard,
            url: value.attr.url,
            blocks: value
                .attr
                .blocks
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            depends: value
                .attr
                .depends
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            ids: value.attr.id.map(|x| x.into_iter().flatten().collect()),
            priority: value.attr.priority,
            severity: value.attr.severity,
            version: value.attr.version,
            component: value.attr.component,
            product: value.attr.product,
            platform: value.attr.platform,
            os: value.attr.os,
            resolution: value.attr.resolution,
            status: value.attr.status,
            target: value.attr.target,

            changed: value
                .change
                .changed
                .map(|x| x.into_iter().map(|x| (x.fields, x.interval)).collect()),
            changed_by: value
                .change
                .changed_by
                .map(|x| x.into_iter().map(|x| (x.fields, x.users)).collect()),
            changed_from: value
                .change
                .changed_from
                .map(|x| x.into_iter().map(|x| (x.field, x.value)).collect()),
            changed_to: value
                .change
                .changed_to
                .map(|x| x.into_iter().map(|x| (x.field, x.value)).collect()),

            comments: value.range.comments,
            votes: value.range.votes,

            created: value.time.created,
            modified: value.time.modified,

            assignee: value
                .user
                .assignee
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attacher: value
                .user
                .attacher
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            cc: value.user.cc,
            commenter: value
                .user
                .commenter
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            flagger: value
                .user
                .flagger
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            qa: value.user.qa,
            reporter: value
                .user
                .reporter
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),

            comment: value.comment.map(|x| x.into_iter().flatten().collect()),
            summary: value.summary.map(|x| x.into_iter().flatten().collect()),

            custom_fields: value.attr.custom_fields.map(|x| {
                x.into_iter()
                    .tuples()
                    .map(|(name, value)| (name, value.into()))
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Search options")]
pub(super) struct Options {
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
    options: Options,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let fields = self.params.query.fields.clone();
        let mut params: Parameters = self.params.into();

        // read attributes from template
        if let Some(path) = self.options.from.as_ref() {
            let template = Parameters::from_path(path)?;
            // command-line parameters override template values
            params = params.merge(template);
        }

        // write attributes to template
        if let Some(path) = self.options.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&params)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        if self.options.browser {
            let url = client.search_url(params)?;
            launch_browser([url])?;
        } else if !self.options.dry_run {
            let items = client.search(params).await?;
            let stdout = stdout().lock();
            render_search(stdout, items, &fields, self.options.json)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_doc(&["bugzilla", "search"]);
    }
}
