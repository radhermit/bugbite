use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::str::FromStr;

use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::objects::RangeOrValue;
use bugbite::output::render_search;
use bugbite::query::Order;
use bugbite::service::bugzilla::search::*;
use bugbite::service::bugzilla::{Bugzilla, FilterField};
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{Merge, RequestTemplate};
use clap::Args;

use crate::service::TemplateOptions;
use crate::utils::launch_browser;

#[derive(Clone, Debug)]
struct ExistsOrMatches(ExistsOrValues<Match>);

impl ExistsOrMatches {
    fn into_inner(self) -> ExistsOrValues<Match> {
        self.0
    }
}

impl FromStr for ExistsOrMatches {
    type Err = bugbite::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Args, Debug)]
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
    alias: Option<Vec<ExistsOrMatches>>,

    /// restrict by attachments
    #[arg(
        short = '@',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    attachments: Option<ExistsOrMatches>,

    /// restrict by blockers
    #[arg(
        short = 'B',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    blocks: Option<Vec<ExistsOrValues<MaybeStdinVec<RangeOrValue<i64>>>>>,

    /// restrict by component
    #[arg(short = 'C', long, value_name = "VALUE[,...]")]
    component: Option<Csv<Match>>,

    /// restrict by custom field
    #[arg(long = "cf", value_name = "NAME[=VALUE]")]
    custom_fields: Option<Vec<String>>,

    /// restrict by dependencies
    #[arg(
        short = 'D',
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        default_missing_value = "true",
    )]
    depends: Option<Vec<ExistsOrValues<MaybeStdinVec<RangeOrValue<i64>>>>>,

    /// restrict by flag
    #[arg(
        short = 'F',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    flags: Option<Vec<ExistsOrMatches>>,

    /// restrict by group
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    groups: Option<Vec<ExistsOrMatches>>,

    /// restrict by ID
    #[arg(long, num_args = 1, value_name = "ID[,...]")]
    id: Option<Vec<ExistsOrValues<MaybeStdinVec<RangeOrValue<i64>>>>>,

    /// restrict by keyword
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
    )]
    keywords: Option<Vec<ExistsOrMatches>>,

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
    see_also: Option<Vec<ExistsOrMatches>>,

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
    tags: Option<Vec<ExistsOrMatches>>,

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
    url: Option<Vec<ExistsOrMatches>>,

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
    whiteboard: Option<Vec<ExistsOrMatches>>,
}

#[derive(Args, Debug)]
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

#[derive(Args, Debug)]
#[clap(next_help_heading = "Range options")]
struct RangeOptions {
    /// restrict by comment count
    #[arg(long)]
    comments: Option<RangeOrValue<u64>>,

    /// restrict by vote count
    #[arg(long)]
    votes: Option<RangeOrValue<u64>>,
}

#[derive(Args, Debug)]
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

#[derive(Args, Debug)]
#[clap(next_help_heading = "Query options")]
struct QueryOptions {
    /// fields to output
    #[arg(short, long, value_name = "FIELD[,...]", default_value = "id,summary")]
    fields: Csv<FilterField>,

    /// limit result count
    #[arg(short, long)]
    limit: Option<usize>,

    /// result starting position
    #[arg(short = 'O', long)]
    offset: Option<usize>,

    /// order query results
    #[arg(short, long, value_name = "FIELD[,...]")]
    order: Option<Csv<Order<OrderField>>>,

    /// enable paging support
    #[arg(long, num_args = 0, default_missing_value = "true")]
    paged: Option<bool>,

    /// search using quicksearch syntax
    #[arg(short = 'S', long, value_name = "QUERY")]
    quicksearch: Option<String>,
}

#[derive(Args, Debug)]
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

#[derive(Args, Debug)]
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
    cc: Option<Vec<ExistsOrMatches>>,

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
    qa: Option<Vec<ExistsOrMatches>>,

    /// user who reported
    #[arg(short = 'R', long, value_name = "USER[,...]")]
    reporter: Option<Vec<Csv<Match>>>,
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Comment options")]
struct CommentOptions {
    /// restrict by comment content
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
#[derive(Args, Debug)]
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

    /// restrict by summary content
    #[clap(value_name = "TERM", help_heading = "Arguments")]
    summary: Option<Vec<MaybeStdinVec<Match>>>,
}

impl From<Params> for Parameters {
    fn from(value: Params) -> Self {
        Self {
            fields: Some(value.query.fields.into_iter().map(Into::into).collect()),
            limit: value.query.limit,
            offset: value.query.offset,
            order: value.query.order.map(|x| x.into_iter().collect()),
            paged: value.query.paged,
            quicksearch: value.query.quicksearch,

            alias: value
                .attr
                .alias
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachments: value.attr.attachments.map(|x| x.into_inner()),
            flags: value
                .attr
                .flags
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            groups: value
                .attr
                .groups
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            keywords: value
                .attr
                .keywords
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            see_also: value
                .attr
                .see_also
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            tags: value
                .attr
                .tags
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            url: value
                .attr
                .url
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            whiteboard: value
                .attr
                .whiteboard
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            blocks: value
                .attr
                .blocks
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            depends: value
                .attr
                .depends
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            ids: value
                .attr
                .id
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
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

            changed: value.change.changed,
            changed_by: value.change.changed_by,
            changed_from: value.change.changed_from,
            changed_to: value.change.changed_to,

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
            cc: value
                .user
                .cc
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            commenter: value
                .user
                .commenter
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            flagger: value
                .user
                .flagger
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            qa: value
                .user
                .qa
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
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
                    .map(|s| {
                        let (name, value) = s.split_once('=').unwrap_or((&s, "true"));
                        (name.to_string(), value.parse().unwrap())
                    })
                    .collect()
            }),
        }
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Search options")]
pub(super) struct Options {
    /// open in browser
    #[arg(short, long)]
    browser: bool,

    /// output in JSON format
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    #[clap(flatten)]
    template: TemplateOptions,

    #[clap(flatten)]
    params: Params,
}

impl Command {
    pub(super) async fn run<W>(self, service: &Bugzilla, f: &mut W) -> anyhow::Result<ExitCode>
    where
        W: IsTerminal + Write,
    {
        let mut request = service.search();

        // read attributes from templates
        if let Some(names) = &self.template.from {
            for name in names {
                request.load_template(name)?;
            }
        }

        // command line parameters override template
        let fields = self.params.query.fields.clone();
        request.params.merge(self.params.into());

        // write attributes to template
        if let Some(name) = self.template.to.as_deref() {
            request.save_template(name)?;
        }

        if self.options.browser {
            let url = request.search_url()?;
            launch_browser([url])?;
        } else if !self.template.dry_run {
            let items = request.stream();
            render_search(f, items, &fields, self.options.json).await?;
        }

        Ok(ExitCode::SUCCESS)
    }
}
