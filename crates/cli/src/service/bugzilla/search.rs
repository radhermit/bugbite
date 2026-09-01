use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::output::render_search;
use bugbite::query::Order;
use bugbite::service::bugzilla::search::*;
use bugbite::service::bugzilla::{Bugzilla, FilterField};
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{Merge, RequestTemplate};
use clap::Args;

use crate::macros::parse_as;
use crate::service::TemplateOptions;
use crate::utils::launch_browser;

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
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    alias: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by attachments
    #[arg(
        short = '@',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
        value_parser = parse_as!(ExistsOrValues<Match>),
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
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    flags: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by group
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    groups: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by ID
    #[arg(short, long, num_args = 1, value_name = "ID[,...]")]
    id: Option<Vec<ExistsOrValues<MaybeStdinVec<RangeOrValue<i64>>>>>,

    /// restrict by keyword
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        default_missing_value = "true",
        value_parser = parse_as!(ExistsOrValues<Match>),
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
    priority: Option<Vec<Csv<Match>>>,

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
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    see_also: Option<Vec<ExistsOrValues<Match>>>,

    /// restrict by severity
    #[arg(long, value_name = "VALUE[,...]")]
    severity: Option<Vec<Csv<Match>>>,

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
        value_parser = parse_as!(ExistsOrValues<Match>),
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
        value_parser = parse_as!(ExistsOrValues<Match>),
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
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    whiteboard: Option<Vec<ExistsOrValues<Match>>>,
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attachment options")]
struct AttachmentOptions {
    /// restrict by creator
    #[arg(long, value_name = "VALUE[,...]")]
    attachment_creator: Option<Vec<Csv<Match>>>,

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

    /// user in CC list
    #[arg(
        long,
        value_name = "USER[,...]",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = parse_as!(ExistsOrValues<Match>),
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
        value_parser = parse_as!(ExistsOrValues<Match>),
    )]
    qa: Option<Vec<ExistsOrValues<Match>>>,

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

impl Merge<Params> for Parameters {
    fn merge(&mut self, other: Params) {
        self.merge(Self {
            fields: Some(other.query.fields.into_iter().collect()),
            limit: other.query.limit,
            offset: other.query.offset,
            order: other.query.order.map(|x| x.into_iter().collect()),
            paged: other.query.paged,
            quicksearch: other.query.quicksearch,

            alias: other.attr.alias,
            attachments: other.attr.attachments,
            flags: other.attr.flags,
            groups: other.attr.groups,
            keywords: other.attr.keywords,
            see_also: other.attr.see_also,
            tags: other.attr.tags,
            url: other.attr.url,
            whiteboard: other.attr.whiteboard,
            blocks: other
                .attr
                .blocks
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            depends: other
                .attr
                .depends
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            ids: other
                .attr
                .id
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            priority: other
                .attr
                .priority
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            severity: other
                .attr
                .severity
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            version: other.attr.version.map(|x| x.into_inner()),
            component: other.attr.component.map(|x| x.into_inner()),
            product: other.attr.product.map(|x| x.into_inner()),
            platform: other.attr.platform.map(|x| x.into_inner()),
            os: other.attr.os.map(|x| x.into_inner()),
            resolution: other.attr.resolution.map(|x| x.into_inner()),
            status: other.attr.status,
            target: other.attr.target.map(|x| x.into_inner()),

            attachment_creator: other
                .attach
                .attachment_creator
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_description: other
                .attach
                .attachment_description
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_filename: other
                .attach
                .attachment_filename
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_mime: other
                .attach
                .attachment_mime
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            attachment_is_obsolete: other.attach.attachment_is_obsolete,
            attachment_is_patch: other.attach.attachment_is_patch,
            attachment_is_private: other.attach.attachment_is_private,

            changed: other.change.changed,
            changed_by: other.change.changed_by,
            changed_from: other.change.changed_from,
            changed_to: other.change.changed_to,

            comments: other.range.comments,
            votes: other.range.votes,

            created: other.time.created,
            updated: other.time.updated,
            closed: other.time.closed,

            assignee: other
                .user
                .assignee
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            cc: other.user.cc,
            commenter: other
                .user
                .commenter
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            flagger: other
                .user
                .flagger
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            qa: other.user.qa,
            reporter: other
                .user
                .reporter
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),

            comment: other
                .comment
                .comment
                .map(|x| x.into_iter().flatten().collect()),
            comment_is_private: other.comment.comment_is_private,
            comment_tag: other
                .comment
                .comment_tag
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),

            summary: other.summary.map(|x| x.into_iter().flatten().collect()),

            custom_fields: other.attr.custom_fields.map(|x| {
                x.into_iter()
                    .map(|s| {
                        let (name, other) = s.split_once('=').unwrap_or((&s, "true"));
                        (name.to_string(), other.parse().unwrap())
                    })
                    .collect()
            }),
        })
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
        request.params.merge(self.params);

        // write attributes to template
        if let Some(name) = &self.template.to {
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
