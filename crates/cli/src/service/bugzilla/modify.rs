use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::modify::{ModifyParams, SetChange};
use bugbite::traits::WebClient;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tempfile::NamedTempFile;
use tracing::info;

use crate::macros::async_block;
use crate::utils::{confirm, launch_editor};

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// modify alias
    #[arg(long)]
    alias: Option<String>,

    /// modify assignee
    #[arg(
        short,
        long,
        value_name = "USER",
        long_help = indoc::indoc! {"
            Assign a bug to a user.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.

            Example:
              - assign bug #123 to yourself: bite m -a @me 123
        "}
    )]
    assigned_to: Option<String>,

    /// add/remove/set blockers
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add, remove, or set blockers.

            Values must be valid IDs for existing bugs.

            Prefixing IDs with `+` or `-` adds or removes bugs from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.
        "}
    )]
    blocks: Option<Vec<SetChange<NonZeroU64>>>,

    /// add/remove CC users
    #[arg(
        long,
        value_name = "USER[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add or remove users from the CC list.

            Values must be email addresses for service users. The alias
            `@me` can also be used for the service's configured user if one
            exists.

            Prefixing values with `+` or `-` adds or removes users from the
            list, respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.

            Examples:
              - add yourself to the CC list: bite m --cc @me 123
              - remove yourself from the CC list: bite m --cc=-@me 123
        "}
    )]
    cc: Option<Vec<SetChange<String>>>,

    /// add a comment
    #[arg(
        short = 'c',
        long,
        num_args = 0..=1,
        default_missing_value = "",
    )]
    comment: Option<String>,

    /// modify component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// modify custom field
    #[arg(long = "cf", num_args = 2, value_names = ["NAME", "VALUE"])]
    custom_fields: Option<Vec<String>>,

    /// add/remove/set dependencies
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add, remove, or set dependencies.

            Values must be valid IDs for existing bugs.

            Prefixing IDs with `+` or `-` adds or removes bugs from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.
        "}
    )]
    depends_on: Option<Vec<SetChange<NonZeroU64>>>,

    /// mark bug as duplicate
    #[arg(short = 'D', long, value_name = "ID", conflicts_with_all = ["status", "resolution"])]
    duplicate_of: Option<NonZeroU64>,

    /// add/remove groups
    #[arg(
        short,
        long,
        value_name = "GROUP[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add or remove groups.

            Values must be valid service groups.

            Prefixing groups with `+` or `-` adds or removes groups from the
            list, respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.
        "}
    )]
    groups: Option<Vec<SetChange<String>>>,

    /// add/remove/set keywords
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "KW[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add, remove, or set keywords.

            Values must be valid keywords.

            Prefixing keywords with `+` or `-` adds or removes them from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.
        "}
    )]
    keywords: Option<Vec<SetChange<String>>>,

    /// modify operating system
    #[arg(long)]
    os: Option<String>,

    /// modify platform
    #[arg(long)]
    platform: Option<String>,

    /// modify priority
    #[arg(long)]
    priority: Option<String>,

    /// modify product
    #[arg(short, long)]
    product: Option<String>,

    /// modify resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// add/remove external bug URLs
    #[arg(
        short = 'U',
        long,
        value_name = "URL[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Add or remove URLs to bugs in external trackers.

            Values must be valid URLs to bugs, issues, or tickets in external
            trackers.

            Prefixing values with `+` or `-` adds or removes URLs from the
            list, respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.
        "}
    )]
    see_also: Option<Vec<SetChange<String>>>,

    /// modify severity
    #[arg(long)]
    severity: Option<String>,

    /// modify status
    #[arg(short, long)]
    status: Option<String>,

    /// modify summary
    #[arg(short = 'S', long)]
    summary: Option<String>,

    /// modify target milestone
    #[arg(short, long, value_name = "MILESTONE")]
    target: Option<String>,

    /// modify URL
    #[arg(short = 'u', long)]
    url: Option<String>,

    /// modify version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// modify whiteboard
    #[arg(short, long)]
    whiteboard: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct Attributes {
    alias: Option<String>,
    assigned_to: Option<String>,
    blocks: Option<Vec<SetChange<NonZeroU64>>>,
    cc: Option<Vec<SetChange<String>>>,
    comment: Option<String>,
    component: Option<String>,
    depends_on: Option<Vec<SetChange<NonZeroU64>>>,
    duplicate_of: Option<NonZeroU64>,
    groups: Option<Vec<SetChange<String>>>,
    keywords: Option<Vec<SetChange<String>>>,
    os: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    product: Option<String>,
    resolution: Option<String>,
    see_also: Option<Vec<SetChange<String>>>,
    severity: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    target: Option<String>,
    url: Option<String>,
    version: Option<String>,
    whiteboard: Option<String>,

    #[serde(flatten)]
    custom_fields: Option<HashMap<String, String>>,
}

impl Attributes {
    fn merge(self, other: Self) -> Self {
        Self {
            alias: self.alias.or(other.alias),
            assigned_to: self.assigned_to.or(other.assigned_to),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            comment: self.comment.or(other.comment),
            component: self.component.or(other.component),
            depends_on: self.depends_on.or(other.depends_on),
            duplicate_of: self.duplicate_of.or(other.duplicate_of),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
            product: self.product.or(other.product),
            resolution: self.resolution.or(other.resolution),
            see_also: self.see_also.or(other.see_also),
            status: self.status.or(other.status),
            severity: self.severity.or(other.severity),
            target: self.target.or(other.target),
            summary: self.summary.or(other.summary),
            url: self.url.or(other.url),
            version: self.version.or(other.version),
            whiteboard: self.whiteboard.or(other.whiteboard),

            custom_fields: self.custom_fields.or(other.custom_fields),
        }
    }

    fn into_params(self, client: &Client) -> anyhow::Result<ModifyParams> {
        let mut params = client.service().modify_params();

        if let Some(value) = self.alias.as_ref() {
            params.alias(value);
        }

        if let Some(value) = self.assigned_to.as_ref() {
            params.assigned_to(value);
        }

        if let Some(values) = self.blocks {
            params.blocks(values);
        }

        if let Some(values) = self.cc {
            params.cc(values);
        }

        if let Some(mut value) = self.comment {
            // interactively create a comment
            if value.trim().is_empty() {
                value = get_comment(value.trim())?;
            }
            params.comment(&value);
        }

        if let Some(value) = self.component.as_ref() {
            params.component(value);
        }

        if let Some(values) = self.custom_fields {
            params.custom_fields(values);
        }

        if let Some(values) = self.depends_on {
            params.depends_on(values);
        }

        if let Some(value) = self.duplicate_of {
            params.duplicate_of(value);
        }

        if let Some(values) = self.groups {
            params.groups(values);
        }

        if let Some(values) = self.keywords {
            params.keywords(values);
        }

        if let Some(value) = self.os.as_ref() {
            params.os(value);
        }

        if let Some(value) = self.platform.as_ref() {
            params.platform(value);
        }

        if let Some(value) = self.priority.as_ref() {
            params.priority(value);
        }

        if let Some(value) = self.product.as_ref() {
            params.product(value);
        }

        if let Some(value) = self.resolution.as_ref() {
            params.resolution(value);
        }

        if let Some(values) = self.see_also {
            params.see_also(values);
        }

        if let Some(value) = self.severity.as_ref() {
            params.severity(value);
        }

        if let Some(value) = self.status.as_ref() {
            params.status(value);
        }

        if let Some(value) = self.target.as_ref() {
            params.target(value);
        }

        if let Some(value) = self.summary.as_ref() {
            params.summary(value);
        }

        if let Some(value) = self.url.as_ref() {
            params.url(value);
        }

        if let Some(value) = self.version.as_ref() {
            params.version(value);
        }

        if let Some(value) = self.whiteboard.as_ref() {
            params.whiteboard(value);
        }

        Ok(params)
    }
}

impl From<Options> for Attributes {
    fn from(value: Options) -> Self {
        Self {
            alias: value.alias,
            assigned_to: value.assigned_to,
            blocks: value.blocks,
            cc: value.cc,
            comment: value.comment,
            component: value.component,
            depends_on: value.depends_on,
            duplicate_of: value.duplicate_of,
            groups: value.groups,
            keywords: value.keywords,
            os: value.os,
            platform: value.platform,
            priority: value.priority,
            product: value.product,
            resolution: value.resolution,
            see_also: value.see_also,
            status: value.status,
            severity: value.severity,
            target: value.target,
            summary: value.summary,
            url: value.url,
            version: value.version,
            whiteboard: value.whiteboard,

            custom_fields: value
                .custom_fields
                .map(|x| x.into_iter().tuples().collect()),
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct Command {
    /// skip service interaction
    #[arg(short = 'n', long, help_heading = "Modify options")]
    dry_run: bool,

    /// reply to specific comments
    #[arg(
        short = 'R',
        long,
        value_name = "ID[,...]",
        value_delimiter = ',',
        help_heading = "Modify options",
        long_help = indoc::indoc! {"
            Interactively reply to specific comments for a given bug.

            Values must be valid comment IDs specific to the bug, starting at 0
            for the description.

            This option forces interactive usage, launching an editor
            pre-populated with the selected comments allowing the user to
            respond in a style reminiscent of threaded messages on a mailing
            list. On completion, the data is used to create a new bug comment.

            Multiple arguments can be specified in a comma-separated list.

            Example:
              - reply to comments 0, 1, and 2 on bug #123: bite m -R 0,4,5 123
        "}
    )]
    reply: Option<Vec<usize>>,

    /// read attributes from a template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = indoc::indoc! {"
            Read modification attributes from a template.

            Value must be the path to a valid modify template file. Templates
            use the TOML format and generally map long option names to values.

            Fields that don't match known bug field names are used for custom
            field modifications.
        "}
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to a template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = indoc::indoc! {"
            Write modification attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating modify
            templates without any service interaction.
        "}
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(
        required = true,
        help_heading = "Arguments",
        long_help = indoc::indoc! {"
            IDs of bugs to modify.

            Taken from standard input when `-`.
        "}
    )]
    ids: Vec<MaybeStdinVec<String>>,
}

/// Interactively create a reply, pulling specified comments for pre-population.
fn get_reply(client: &Client, id: &str, comment_ids: &[usize]) -> anyhow::Result<String> {
    let comments = async_block!(client.comment(&[id], None))?
        .into_iter()
        .next()
        .expect("invalid comments response");
    if comments.is_empty() {
        anyhow::bail!("reply invalid, bug #{id} has no comments")
    }

    let mut data = vec![];
    for id in comment_ids {
        let Some(comment) = comments.get(*id) else {
            anyhow::bail!("reply invalid, nonexistent comment #{id}");
        };
        data.push(comment);
    }
    let data = data.iter().map(|x| x.reply()).join("\n\n");

    // interactively edit the comment
    get_comment(&data)
}

/// Interactively edit a comment.
fn get_comment(data: &str) -> anyhow::Result<String> {
    let temp_file = NamedTempFile::new()?;
    if !data.is_empty() {
        fs::write(&temp_file, data)?;
    }

    loop {
        let status = launch_editor(&temp_file)?;
        if !status.success() {
            anyhow::bail!("failed editing reply content");
        }
        let comment = fs::read_to_string(&temp_file)?;
        if comment != data || confirm("No changes made to comment, submit anyway?", false)? {
            return Ok(comment);
        }
    }
}

impl Command {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();
        let mut attrs: Attributes = self.options.into();

        // read modification attributes from a template
        if let Some(path) = self.from.as_ref() {
            let data = fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("failed loading template: {path}: {e}"))?;
            let template = toml::from_str(&data)
                .map_err(|e| anyhow::anyhow!("failed parsing template: {path}: {e}"))?;
            // command-line options override template options
            attrs = attrs.merge(template);
        };

        // write modification attributes to a template
        if let Some(path) = self.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&attrs)?;
                fs::write(path, data)?;
            }
        }

        let mut params = attrs.into_params(client)?;

        // interactively create a reply
        if let Some(values) = self.reply.as_ref() {
            if ids.len() > 1 {
                anyhow::bail!("reply invalid, targeting multiple bugs");
            }
            let comment = get_reply(client, ids[0], values)?;
            params.comment(comment.trim());
        }

        if !self.dry_run {
            let changes = async_block!(client.modify(ids, params))?;
            for change in changes {
                info!("{change}");
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
