use std::fs;
use std::io::stdout;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::Context;
use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::objects::RangeOrValue;
use bugbite::query::Order;
use bugbite::service::bugzilla::Service;
use bugbite::service::bugzilla::{
    search::{ChangeField, Match, OrderField, Parameters},
    FilterField,
};
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{RequestMerge, RequestSend};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use crossterm::style::Stylize;
use itertools::Itertools;
use strum::VariantNames;

use crate::service::output::render_search;
use crate::utils::{confirm, launch_browser};

/// Parse a string into a ChangeField, adding possible values to the error on failure.
fn change_field(s: &str) -> anyhow::Result<ChangeField> {
    s.parse().map_err(|_| {
        let possible = ChangeField::VARIANTS.iter().map(|s| s.green()).join(", ");
        anyhow::anyhow!("invalid change field: {s}\n  [possible values: {possible}]")
    })
}

#[derive(Clone)]
struct Changed {
    fields: Vec<ChangeField>,
    interval: RangeOrValue<TimeDeltaOrStatic>,
}

impl FromStr for Changed {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_fields, time) = s.split_once('=').unwrap_or((s, "<now"));
        Ok(Self {
            fields: raw_fields.split(',').map(change_field).try_collect()?,
            interval: time.parse()?,
        })
    }
}

#[derive(Clone)]
struct ChangedBy {
    fields: Vec<ChangeField>,
    users: Vec<String>,
}

impl FromStr for ChangedBy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((raw_fields, users)) = s.split_once('=') else {
            anyhow::bail!("missing value");
        };

        Ok(Self {
            fields: raw_fields.split(',').map(change_field).try_collect()?,
            users: users.split(',').map(|s| s.to_string()).collect(),
        })
    }
}

#[derive(Clone)]
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

#[derive(Args)]
#[clap(next_help_heading = "Attribute options")]
struct AttributeOptions {
    /// restrict by alias
    #[arg(
        short = 'A',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    alias: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by attachments
    #[arg(
        short = '@',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    attachments: Option<ExistsOrValues<Match>>,

    /// restrict by blockers
    #[arg(
        short = 'B',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    blocks: Option<Vec<ExistsOrValues<MaybeStdinVec<i64>>>>,

    /// restrict by component
    #[arg(short = 'C', long, value_name = "VALUE[,...]")]
    component: Option<Csv<Match>>,

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
    #[arg(long, num_args = 1, value_name = "ID[,...]", value_delimiter = ',')]
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

    /// restrict by operating system
    #[arg(long, value_name = "VALUE[,...]")]
    os: Option<Csv<Match>>,

    /// restrict by platform
    #[arg(long, value_name = "VALUE[,...]")]
    platform: Option<Csv<Match>>,

    /// restrict by priority
    #[arg(long, value_name = "VALUE[,...]")]
    priority: Option<Csv<Match>>,

    /// restrict by product
    #[arg(short, long, value_name = "VALUE[,...]")]
    product: Option<Csv<Match>>,

    /// restrict by resolution
    #[arg(short, long, value_name = "VALUE[,...]")]
    resolution: Option<Csv<Match>>,

    /// restrict by tracker URLs
    #[arg(
        short = 'U',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    see_also: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by severity
    #[arg(long, value_name = "VALUE[,...]")]
    severity: Option<Csv<Match>>,

    /// restrict by status
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        num_args = 1
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
    #[arg(short = 'T', long, value_name = "VALUE[,...]")]
    target: Option<Csv<Match>>,

    /// restrict by URL
    #[arg(
        long,
        value_name = "VALUE[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
    )]
    url: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by version
    #[arg(short = 'V', long, value_name = "VALUE[,...]")]
    version: Option<Csv<Match>>,

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

#[derive(Args)]
#[clap(next_help_heading = "Attachment options")]
struct AttachmentOptions {
    /// restrict by description
    #[arg(long, value_name = "VALUE[,...]")]
    attachment_description: Option<Vec<Csv<Match>>>,

    /// restrict by file name
    #[arg(long, value_name = "VALUE[,...]")]
    attachment_filename: Option<Vec<Csv<Match>>>,

    /// restrict by MIME type
    #[arg(long, value_name = "VALUE[,...]")]
    attachment_mime: Option<Vec<Csv<Match>>>,

    /// restrict by obsolete status
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    attachment_is_obsolete: Option<bool>,

    /// restrict by patch status
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    attachment_is_patch: Option<bool>,

    /// restrict by private status
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    attachment_is_private: Option<bool>,
}

#[derive(Args)]
#[clap(next_help_heading = "Range options")]
struct RangeOptions {
    /// restrict by comments
    #[arg(long)]
    comments: Option<RangeOrValue<u64>>,

    /// restrict by votes
    #[arg(long)]
    votes: Option<RangeOrValue<u64>>,
}

#[derive(Args)]
#[clap(next_help_heading = "Change options")]
struct ChangeOptions {
    /// fields changed within time interval
    #[arg(long, value_name = "FIELD[,...][=TIME]")]
    changed: Option<Vec<Changed>>,

    /// fields changed by users
    #[arg(long, value_name = "FIELD[,...]=USER[,...]")]
    changed_by: Option<Vec<ChangedBy>>,

    /// fields changed from value
    #[arg(long, value_name = "FIELD=VALUE")]
    changed_from: Option<Vec<ChangedValue>>,

    /// fields changed to value
    #[arg(long, value_name = "FIELD=VALUE")]
    changed_to: Option<Vec<ChangedValue>>,
}

#[derive(Args)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(short, long, value_name = "FIELD[,...]", default_value = "id,summary")]
    fields: Csv<FilterField>,

    /// limit result count
    #[arg(short, long)]
    limit: Option<u64>,

    /// order query results
    #[arg(short, long, value_name = "FIELD[,...]")]
    order: Option<Csv<Order<OrderField>>>,

    /// search using quicksearch syntax
    #[arg(short = 'S', long, value_name = "QUERY")]
    quicksearch: Option<String>,
}

#[derive(Args)]
#[clap(next_help_heading = "Time options")]
struct TimeOptions {
    /// restrict by creation time
    #[arg(short, long, value_name = "TIME")]
    created: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by update time
    #[arg(short, long, value_name = "TIME")]
    updated: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// restrict by closed time
    #[arg(long, value_name = "TIME")]
    closed: Option<RangeOrValue<TimeDeltaOrStatic>>,
}

#[derive(Args)]
#[clap(next_help_heading = "User options")]
struct UserOptions {
    /// user is assignee
    #[arg(short, long, value_name = "USER[,...]")]
    assignee: Option<Vec<Csv<Match>>>,

    /// user created attachment
    #[arg(long, value_name = "USER[,...]")]
    attacher: Option<Vec<Csv<Match>>>,

    /// user in CC list
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

    /// user who set flag
    #[arg(long, value_name = "USER[,...]")]
    flagger: Option<Vec<Csv<Match>>>,

    /// user is QA contact
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

#[derive(Args)]
#[clap(next_help_heading = "Comment options")]
struct CommentOptions {
    /// strings to search for in comments
    #[clap(long, value_name = "TERM")]
    comment: Option<Vec<MaybeStdinVec<Match>>>,

    /// restrict by private status
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    comment_is_private: Option<bool>,

    /// restrict by tag
    #[arg(long, value_name = "VALUE[,...]")]
    comment_tag: Option<Vec<Csv<Match>>>,
}

/// Available search parameters.
///
/// See https://bugzilla.readthedocs.io/en/latest/api/core/v1/bug.html#search-bugs for more
/// information.
#[derive(Args)]
struct Params {
    #[clap(flatten)]
    query: QueryOptions,

    #[clap(flatten)]
    attr: AttributeOptions,

    #[clap(flatten)]
    attach: AttachmentOptions,

    #[clap(flatten)]
    range: RangeOptions,

    #[clap(flatten)]
    change: ChangeOptions,

    #[clap(flatten)]
    time: TimeOptions,

    #[clap(flatten)]
    user: UserOptions,

    #[clap(flatten)]
    comment: CommentOptions,

    /// summary strings to search for
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
            priority: value.attr.priority.map(|x| x.into_inner()),
            severity: value.attr.severity.map(|x| x.into_inner()),
            version: value.attr.version.map(|x| x.into_inner()),
            component: value.attr.component.map(|x| x.into_inner()),
            product: value.attr.product.map(|x| x.into_inner()),
            platform: value.attr.platform.map(|x| x.into_inner()),
            os: value.attr.os.map(|x| x.into_inner()),
            resolution: value.attr.resolution.map(|x| x.into_inner()),
            status: value.attr.status,
            target: value.attr.target.map(|x| x.into_inner()),

            attachment_description: value
                .attach
                .attachment_description
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_filename: value
                .attach
                .attachment_filename
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_mime: value
                .attach
                .attachment_mime
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_is_obsolete: value.attach.attachment_is_obsolete,
            attachment_is_patch: value.attach.attachment_is_patch,
            attachment_is_private: value.attach.attachment_is_private,

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
            updated: value.time.updated,
            closed: value.time.closed,

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

            comment: value
                .comment
                .comment
                .map(|x| x.into_iter().flatten().collect()),
            comment_is_private: value.comment.comment_is_private,
            comment_tag: value
                .comment
                .comment_tag
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),

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

#[derive(Args)]
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

#[derive(Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run(self, service: &Service) -> anyhow::Result<ExitCode> {
        let mut request = service.search();

        // read attributes from template
        if let Some(path) = self.options.from.as_deref() {
            request.merge(path)?;
        }

        // command line parameters override template
        let fields = self.params.query.fields.clone();
        request.merge(self.params)?;

        // write attributes to template
        if let Some(path) = self.options.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&request)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        if self.options.browser {
            let url = request.search_url()?;
            launch_browser([url])?;
        } else if !self.options.dry_run {
            let items = request.send().await?;
            let stdout = stdout().lock();
            render_search(stdout, items, &fields, self.options.json)?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
