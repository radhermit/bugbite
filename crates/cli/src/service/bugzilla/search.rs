use std::process::ExitCode;
use std::str::FromStr;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::objects::RangeOrEqual;
use bugbite::query::Order;
use bugbite::service::bugzilla::{
    search::{ChangeField, EnabledOrDisabled, ExistsField, Match, OrderField},
    BugField,
};
use bugbite::time::TimeDeltaIso8601;
use bugbite::traits::WebClient;
use camino::Utf8PathBuf;
use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Args, ValueHint};
use itertools::Itertools;
use strum::VariantNames;

use crate::service::args::ExistsOrArray;
use crate::service::output::render_search;
use crate::utils::{launch_browser, wrapped_doc};

#[derive(Debug, Clone)]
struct Changed {
    fields: Vec<ChangeField>,
    interval: TimeDeltaIso8601,
}

impl FromStr for Changed {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((raw_fields, time)) = s.split_once('=') else {
            anyhow::bail!("missing time interval");
        };

        let mut fields = vec![];
        for s in raw_fields.split(',') {
            let field = s
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid change field: {s}"))?;
            fields.push(field);
        }

        Ok(Self {
            fields,
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

        let mut fields = vec![];
        for s in raw_fields.split(',') {
            let field = s
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid change field: {s}"))?;
            fields.push(field);
        }

        Ok(Self {
            fields,
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

        let field = field
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid change field: {field}"))?;

        Ok(Self {
            field,
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
            can be specified in a comma-separated list, matching if any of the
            specified values match.

            Examples:
            - contains `value`
            > bite s --alias value

            - contains `value1` or `value2`
            > bite s --alias value1,value1

            Values can use match operator prefixes to alter their query
            application. Note that some operators may need to be escaped when
            used in the shell environment.

            Examples:
            - doesn't contain `value`
            > bite s --alias !#value

            - equals `value`
            > bite s --alias =#value

            - doesn't equal `value`
            > bite s --alias !=#value

            - matches regex
            > bite s --alias r#test?.+

            - doesn't match regex
            > bite s --alias !r#test?.+
        "#)
    )]
    alias: Option<ExistsOrArray<Match>>,

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
            > bite s --attachments !#value

            - equals `value`
            > bite s --attachments =#value

            - doesn't equal `value`
            > bite s --attachments !=#value

            - matches regex
            > bite s --attachments r#test?.+

            - doesn't match regex
            > bite s --attachments !r#test?.+

            Multiple values can be specified in a comma-separated list and will
            match if any of the specified values match.

            Example:
            - equals `test1` or `test2`
            > bite s --attachments =#test1,=#test2
        "#)
    )]
    attachments: Option<ExistsOrArray<Match>>,

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
            be specified in a comma-separated list, matching if all of the
            specified blockers match.

            Examples:
            - blocked on 10
            > bite s --blocks 10

            - blocked on 10 and 11
            > bite s --blocks 10,11

            Values can also use `-` or `+` prefixes to manipulate blocker
            existence for the query.

            Examples:
            - isn't blocked on 10
            > bite s --blocks=-10

            - blocked on 10 and 11
            > bite s --blocks +10,11

            - blocked on 10 but not 11
            > bite s --blocks 10,-11

            Values are taken from standard input when `-`.
        ")
    )]
    blocks: Option<ExistsOrArray<MaybeStdinVec<EnabledOrDisabled<u64>>>>,

    /// restrict by component
    #[arg(short = 'C', long, value_delimiter = ',')]
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
            be specified in a comma-separated list, matching if all of the
            specified dependencies match.

            Examples:
            - depends on 10
            > bite s --depends 10

            - depends on 10 and 11
            > bite s --depends 10,11

            Values can also use `-` or `+` prefixes to manipulate dependency
            existence for the query.

            Examples:
            - doesn't depend on 10
            > bite s --depends=-10

            - depends on 10 and 11
            > bite s --depends +10,11

            - depends on 10 but not 11
            > bite s --depends 10,-11

            Values are taken from standard input when `-`.
        ")
    )]
    depends: Option<ExistsOrArray<MaybeStdinVec<EnabledOrDisabled<u64>>>>,

    /// restrict by flag
    #[arg(
        short = 'F',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    flags: Option<ExistsOrArray<MaybeStdinVec<Match>>>,

    /// restrict by group
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    groups: Option<ExistsOrArray<MaybeStdinVec<Match>>>,

    /// restrict by ID
    #[arg(long, value_delimiter = ',')]
    id: Option<Vec<MaybeStdinVec<u64>>>,

    /// restrict by keyword
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    keywords: Option<ExistsOrArray<MaybeStdinVec<Match>>>,

    /// restrict by OS
    #[arg(long, value_name = "VALUE[,...]", value_delimiter = ',')]
    os: Option<Vec<Match>>,

    /// restrict by platform
    #[arg(long, value_name = "VALUE[,...]", value_delimiter = ',')]
    platform: Option<Vec<Match>>,

    /// restrict by priority
    #[arg(long, value_name = "VALUE[,...]", value_delimiter = ',')]
    priority: Option<Vec<Match>>,

    /// restrict by product
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    product: Option<Vec<Match>>,

    /// restrict by resolution
    #[arg(short, long, value_name = "VALUE[,...]", value_delimiter = ',')]
    resolution: Option<Vec<Match>>,

    /// restrict by external URLs
    #[arg(
        short = 'U',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    see_also: Option<ExistsOrArray<Match>>,

    /// restrict by severity
    #[arg(long, value_name = "VALUE[,...]", value_delimiter = ',')]
    severity: Option<Vec<Match>>,

    /// restrict by status
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Restrict bugs by status.

            The aliases `@open`, `@closed`, and `@all` can be used to search for
            open, closed, and all bugs, respectively.
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
    tags: Option<ExistsOrArray<Match>>,

    /// restrict by target milestone
    #[arg(short = 'T', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    target: Option<Vec<Match>>,

    /// restrict by URL
    #[arg(
        short = 'u',
        long,
        value_name = "VALUE[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    url: Option<ExistsOrArray<Match>>,

    /// restrict by version
    #[arg(short = 'V', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    version: Option<Vec<Match>>,

    /// restrict by whiteboard
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    whiteboard: Option<ExistsOrArray<Match>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Range options")]
struct RangeOptions {
    /// restrict by comments
    #[arg(long)]
    comments: Option<RangeOrEqual<u64>>,

    /// restrict by votes
    #[arg(long)]
    votes: Option<RangeOrEqual<u64>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "Change options")]
struct ChangeOptions {
    /// fields changed at this time or later
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
        value_delimiter = ',',
        default_value = "id,summary",
        hide_possible_values = true,
        value_parser = PossibleValuesParser::new(BugField::VARIANTS)
                .map(|s| s.parse::<BugField>().unwrap()),
        long_help = wrapped_doc!("
            Restrict the data fields returned by the query.

            By default, only the id and summary fields are returned. This
            can be altered by specifying a custom list of fields which will
            change the output format to a space separated list of the
            field values for each item.

            Possible values: {}",
            BugField::VARIANTS.join(", ")
        )
    )]
    fields: Vec<BugField>,

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
        value_delimiter = ',',
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
    order: Option<Vec<Order<OrderField>>>,

    /// search using query grammar
    #[arg(short = 'Q', long)]
    query: Option<String>,

    /// search using quicksearch syntax
    #[arg(
        short = 'S',
        long,
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
    created: Option<RangeOrEqual<TimeDeltaIso8601>>,

    /// restrict by modification time
    #[arg(short, long, value_name = "TIME")]
    modified: Option<RangeOrEqual<TimeDeltaIso8601>>,
}

#[derive(Debug, Args)]
#[clap(next_help_heading = "User options")]
struct UserOptions {
    /// user is the assignee
    #[arg(short, long, value_name = "USER[,...]", value_delimiter = ',')]
    assignee: Option<Vec<Match>>,

    /// user created attachment
    #[arg(long, value_name = "USER[,...]", value_delimiter = ',')]
    attachers: Option<Vec<Match>>,

    /// user in the CC list
    #[arg(
        long,
        value_name = "USER[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    cc: Option<ExistsOrArray<Match>>,

    /// user who commented
    #[arg(long, value_name = "USER[,...]", value_delimiter = ',')]
    commenters: Option<Vec<Match>>,

    /// user who set a flag
    #[arg(long, value_name = "USER[,...]", value_delimiter = ',')]
    flaggers: Option<Vec<Match>>,

    /// user is the QA contact
    #[arg(
        long,
        value_name = "USER[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    qa: Option<ExistsOrArray<Match>>,

    /// user who reported
    #[arg(short = 'R', long, value_name = "USER[,...]", value_delimiter = ',')]
    reporter: Option<Vec<Match>>,
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

    /// skip service interaction
    #[arg(short = 'n', long, help_heading = "Search options")]
    dry_run: bool,

    /// read attributes from a template
    #[arg(
        long,
        help_heading = "Search options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Read search attributes from a template.

            Value must be the path to a valid search template file.
            Templates use the TOML format and generally map long option names to
            values.
        ")
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to a template
    #[arg(
        long,
        help_heading = "Search options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Write search attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating search
            templates without any service interaction.
        ")
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        // TODO: implement a custom serde serializer to convert structs to URL parameters
        let mut query = client.service().search_query();
        let params = self.params;

        if let Some(values) = params.change.changed {
            for change in values {
                query.changed(change.fields.iter().map(|f| (*f, &change.interval)));
            }
        }
        if let Some(values) = params.change.changed_by {
            for change in values {
                query.changed_by(change.fields.iter().map(|f| (*f, &change.users)));
            }
        }
        if let Some(values) = params.change.changed_from {
            query.changed_from(values.into_iter().map(|c| (c.field, c.value)));
        }
        if let Some(values) = params.change.changed_to {
            query.changed_to(values.into_iter().map(|c| (c.field, c.value)));
        }
        if let Some(values) = params.user.assignee {
            query.assignee(values);
        }
        if let Some(value) = params.query.limit {
            query.limit(value);
        }
        if let Some(value) = params.time.created {
            query.created(value);
        }
        if let Some(value) = params.time.modified {
            query.modified(value);
        }
        if let Some(values) = params.query.order {
            query.order(values)?;
        }
        if let Some(values) = params.user.attachers {
            query.attachers(values);
        }
        if let Some(values) = params.user.commenters {
            query.commenters(values);
        }
        if let Some(values) = params.user.flaggers {
            query.flaggers(values);
        }
        if let Some(values) = params.attr.custom_fields {
            query.custom_fields(values.into_iter().tuples());
        }
        if let Some(values) = params.attr.priority {
            query.priority(values);
        }
        if let Some(values) = params.attr.severity {
            query.severity(values);
        }
        if let Some(values) = params.attr.version {
            query.version(values);
        }
        if let Some(values) = params.attr.component {
            query.component(values);
        }
        if let Some(values) = params.attr.product {
            query.product(values);
        }
        if let Some(values) = params.attr.platform {
            query.platform(values);
        }
        if let Some(values) = params.attr.os {
            query.os(values);
        }
        if let Some(values) = params.attr.see_also {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::SeeAlso, value),
                ExistsOrArray::Array(values) => query.see_also(values),
            }
        }
        if let Some(values) = params.user.reporter {
            query.reporter(values);
        }
        if let Some(values) = params.attr.resolution {
            query.resolution(values);
        }
        if let Some(values) = params.attr.status {
            query.status(values);
        }
        if let Some(values) = params.attr.tags {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Tags, value),
                ExistsOrArray::Array(values) => query.tags(values),
            }
        }
        if let Some(values) = params.attr.target {
            query.target(values);
        }
        if let Some(values) = params.attr.whiteboard {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Whiteboard, value),
                ExistsOrArray::Array(values) => query.whiteboard(values),
            }
        }
        if let Some(values) = params.attr.url {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Url, value),
                ExistsOrArray::Array(values) => query.url(values),
            }
        }
        if let Some(value) = params.range.votes {
            query.votes(value);
        }
        if let Some(value) = params.range.comments {
            query.comments(value);
        }
        if let Some(values) = params.attr.alias {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Alias, value),
                ExistsOrArray::Array(values) => query.alias(values),
            }
        }
        if let Some(values) = params.attr.attachments {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Attachments, value),
                ExistsOrArray::Array(values) => query.attachments(values),
            }
        }
        if let Some(values) = params.attr.id {
            query.id(values.into_iter().flatten());
        }
        if let Some(values) = params.comment {
            query.comment(values.into_iter().flatten());
        }
        if let Some(values) = params.summary {
            query.summary(values.into_iter().flatten());
        }
        if let Some(values) = params.attr.flags {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Flags, value),
                ExistsOrArray::Array(values) => query.flags(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.attr.groups {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Groups, value),
                ExistsOrArray::Array(values) => query.groups(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.attr.keywords {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Keywords, value),
                ExistsOrArray::Array(values) => query.keywords(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.user.cc {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Cc, value),
                ExistsOrArray::Array(values) => query.cc(values),
            }
        }
        if let Some(values) = params.user.qa {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Qa, value),
                ExistsOrArray::Array(values) => query.qa(values),
            }
        }
        if let Some(values) = params.attr.blocks {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::Blocks, value),
                ExistsOrArray::Array(values) => query.blocks(values.into_iter().flatten()),
            }
        }
        if let Some(values) = params.attr.depends {
            match values {
                ExistsOrArray::Exists(value) => query.exists(ExistsField::DependsOn, value),
                ExistsOrArray::Array(values) => query.depends(values.into_iter().flatten()),
            }
        }
        if let Some(value) = params.query.quicksearch {
            query.quicksearch(value);
        }

        let fields = &params.query.fields;
        query.fields(fields.iter().copied());

        if self.browser {
            let url = client.search_url(query)?;
            launch_browser([url])?;
        } else if !self.dry_run {
            let bugs = client.search(query).await?;
            render_search(bugs, fields)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
