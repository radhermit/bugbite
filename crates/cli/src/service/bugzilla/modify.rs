use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::process::ExitCode;
use std::str::FromStr;
use std::{fmt, fs};

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::objects::bugzilla::Flag;
use bugbite::objects::Range;
use bugbite::service::bugzilla::modify::{ModifyParams, SetChange};
use bugbite::traits::{Contains, WebClient};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none, DeserializeFromStr, SerializeDisplay};
use tempfile::NamedTempFile;
use tracing::info;

use crate::utils::{confirm, launch_editor, wrapped_doc};

#[derive(Debug, Clone)]
enum RangeOrSet<T: FromStr + PartialOrd + Eq + Hash> {
    Range(Range<T>),
    Set(HashSet<T>),
}

impl<T: FromStr + PartialOrd + Eq + Hash> FromStr for RangeOrSet<T>
where
    <T as FromStr>::Err: fmt::Display + fmt::Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse() {
            Ok(Self::Range(value))
        } else {
            let mut set = HashSet::new();
            for x in s.split(',') {
                let value = x
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid value: {e}"))?;
                set.insert(value);
            }
            Ok(Self::Set(set))
        }
    }
}

impl<T: FromStr + PartialOrd + Eq + Hash> Contains<T> for RangeOrSet<T> {
    fn contains(&self, obj: &T) -> bool {
        match self {
            Self::Range(value) => value.contains(obj),
            Self::Set(value) => value.contains(obj),
        }
    }
}

#[derive(DeserializeFromStr, SerializeDisplay, Debug, Clone)]
struct CommentPrivacy<T: FromStr + PartialOrd + Eq + Hash> {
    raw: String,
    container: Option<RangeOrSet<T>>,
    is_private: Option<bool>,
}

impl<T: FromStr + PartialOrd + Eq + Hash> CommentPrivacy<T> {
    /// Determine if newly created comments should be created private.
    fn created_private(&self) -> bool {
        self.container.is_none() && self.is_private.unwrap_or_default()
    }
}

impl<T: FromStr + PartialOrd + Eq + Hash> FromStr for CommentPrivacy<T>
where
    <T as FromStr>::Err: fmt::Display + fmt::Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (container, is_private) = if let Some((ids, value)) = s.split_once(':') {
            (Some(ids.parse()?), Some(value.parse()?))
        } else if let Ok(value) = s.parse::<bool>() {
            (None, Some(value))
        } else {
            (Some(s.parse()?), None)
        };

        Ok(Self {
            raw: s.to_string(),
            container,
            is_private,
        })
    }
}

impl<T: FromStr + PartialOrd + Eq + Hash> fmt::Display for CommentPrivacy<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.raw.fmt(f)
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// add/remove/set aliases
    #[arg(
        short = 'A',
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add, remove, or set aliases.

            Values must be unique when adding or setting and are ignored when
            missing when removing.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.

            Examples modifying bug 10:
            - add `a1`
            > bite m 10 --alias +a1

            - add `a2` and remove `a1`
            > bite m 10 --alias +a2,-a1

            - set to `a3`
            > bite m 10 --alias a3
        ")
    )]
    alias: Option<Vec<SetChange<String>>>,

    /// modify assignee
    #[arg(
        short,
        long,
        value_name = "USER",
        num_args = 0..=1,
        default_missing_value = "",
        long_help = wrapped_doc!(r#"
            Assign a bug to a user.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.

            No argument or an empty string will reset the field to the default
            for target component.

            Example modifying bug 123:
            - assign to yourself
            > bite m 123 --assignee @me

            - reset to default
            > bite m 123 --assignee ""
        "#)
    )]
    assignee: Option<String>,

    /// add/remove/set blockers
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add, remove, or set blockers.

            Values must be valid IDs for existing bugs.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.

            Examples modifying bug 10:
            - add 1
            > bite m 10 --blocks +1

            - add 2 and remove 1
            > bite m 10 --blocks +2,-1

            - set to 3
            > bite m 10 --blocks 3
        ")
    )]
    blocks: Option<Vec<SetChange<u64>>>,

    /// add/remove CC users
    #[arg(
        long,
        value_name = "USER[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add or remove users from the CC list.

            Values must be email addresses for service users. The alias
            `@me` can also be used for the service's configured user if one
            exists.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.

            Examples modifying bug 123:
            - add yourself to the CC list
            > bite m 123 --cc @me

            - remove yourself from the CC list
            > bite m 123 --cc=-@me
        ")
    )]
    cc: Option<Vec<SetChange<String>>>,

    /// add a comment
    #[arg(
        short = 'c',
        long,
        num_args = 0..=1,
        conflicts_with = "reply",
        default_missing_value = "",
        long_help = wrapped_doc!("
            Add a comment.

            When no comment argument is specified, an editor is launched
            allowing for interactive entry.
        ")
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
        long_help = wrapped_doc!("
            Add, remove, or set dependencies.

            Values must be valid IDs for existing bugs.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.

            Examples modifying bug 10:
            - add 1
            > bite m 10 --depends +1

            - add 2 and remove 1
            > bite m 10 --depends +2,-1

            - set to 3
            > bite m 10 --depends 3
        ")
    )]
    depends: Option<Vec<SetChange<u64>>>,

    /// mark bug as duplicate
    #[arg(short = 'D', long, value_name = "ID", conflicts_with_all = ["status", "resolution"])]
    duplicate_of: Option<u64>,

    /// add/remove flags
    #[arg(
        short = 'F',
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!(r#"
            Add or remove flags.

            Values must be valid flags composed of the flag name followed by its
            status. Supported statuses include `+`, `-`, and `?`. In addition,
            the special status `X` removes a flag.

            Multiple arguments can be specified in a comma-separated list.

            Examples modifying bug 10:
            - add `test?`
            > bite m 10 --flags "test?"

            - add `check+` and remove `test?`
            > bite m 10 --flags check+,testX
        "#)
    )]
    flags: Option<Vec<Flag>>,

    /// add/remove groups
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add or remove groups.

            Values must be valid service groups.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.

            Examples modifying bug 10:
            - add `admin`
            > bite m 10 --groups +admin

            - add `test` and remove `admin`
            > bite m 10 --groups +test,-admin
        ")
    )]
    groups: Option<Vec<SetChange<String>>>,

    /// add/remove/set keywords
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add, remove, or set keywords.

            Values must be valid keywords.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values are treated as set values and
            override the entire list, ignoring any prefixed values.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.

            Examples modifying bug 10:
            - add `key`
            > bite m 10 --keywords +key

            - add `test` and remove `key`
            > bite m 10 --keywords +test,-key

            - set to `verify`
            > bite m 10 --keywords verify
        ")
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

    /// modify comment privacy
    #[arg(
        short = 'P',
        long,
        value_name = "VALUE",
        num_args = 0..=1,
        default_missing_value = "true",
        long_help = wrapped_doc!("
            Modify the privacy of comments.

            This option controls modifying the privacy of both existing comments
            and those currently being created via the `--comment` or `--reply`
            options.

            To modify existing comments, the value must be comma-separated
            comment IDs local to the specified bug ID starting at 0 for the bug
            description or a range of comment IDs. An optional suffix consisting
            of boolean value in the form of `:true` or `:false` can be included
            to enable or disable all comment privacy respectively. Without this
            suffix, the privacy of all matching comments is toggled.

            To modify comments being created, either no argument can be used to
            enable privacy or an explicit boolean.

            Examples modifying bug 10:
            - toggle comment 1 privacy
            > bite m 10 --private-comment 1

            - toggle comment 1 and 2 privacy
            > bite m 10 --private-comment 1,2

            - toggle all comment privacy
            > bite m 10 --private-comment ..

            - disable comment 1 and 2 privacy
            > bite m 10 --private-comment 1,2:false

            - mark comments 2-5 private
            > bite m 10 --private-comment 2..=5:true

            - mark created comment private
            > bite m 10 --comment --private-comment

            - mark created reply private
            > bite m 10 --reply --private-comment
        ")
    )]
    private_comment: Option<CommentPrivacy<usize>>,

    /// modify product
    #[arg(short, long)]
    product: Option<String>,

    /// modify QA contact
    #[arg(
        long,
        value_name = "USER",
        num_args = 0..=1,
        default_missing_value = "",
        long_help = wrapped_doc!(r#"
            Assign a QA contact for the bug.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.

            No argument or an empty string will reset the field to the default
            for target component.

            Examples modifying bug 123:
            - assign to yourself
            > bite m 123 --qa @me

            - reset to default
            > bite m 123 --qa
        "#)
    )]
    qa: Option<String>,

    /// modify resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// add/remove bug URLs
    #[arg(
        short = 'U',
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Add or remove URLs to bugs in external trackers.

            Values must be valid URLs to bugs, issues, or tickets in external
            trackers or IDs to existing bugs for the targeted service.

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values will be added to the list.

            Multiple arguments can be specified in a comma-separated list.

            Examples modifying bug 10:
            - add URL to bug 2
            > bite m 10 --see-also 2

            - add bug 3 URL and remove 2
            > bite m 10 --see-also=+3,-2

            - add URL to external bug
            > bite m 10 --see-also https://url/to/bug/5
        ")
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
    #[arg(short = 'T', long, value_name = "MILESTONE")]
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
    alias: Option<Vec<SetChange<String>>>,
    assignee: Option<String>,
    blocks: Option<Vec<SetChange<u64>>>,
    cc: Option<Vec<SetChange<String>>>,
    comment: Option<String>,
    component: Option<String>,
    depends: Option<Vec<SetChange<u64>>>,
    duplicate_of: Option<u64>,
    flags: Option<Vec<Flag>>,
    groups: Option<Vec<SetChange<String>>>,
    keywords: Option<Vec<SetChange<String>>>,
    os: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    private_comment: Option<CommentPrivacy<usize>>,
    product: Option<String>,
    qa: Option<String>,
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
            assignee: self.assignee.or(other.assignee),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            comment: self.comment.or(other.comment),
            component: self.component.or(other.component),
            depends: self.depends.or(other.depends),
            duplicate_of: self.duplicate_of.or(other.duplicate_of),
            flags: self.flags.or(other.flags),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
            private_comment: self.private_comment.or(other.private_comment),
            product: self.product.or(other.product),
            qa: self.qa.or(other.qa),
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

    async fn into_params<'a, S>(
        self,
        client: &'a Client,
        ids: &[S],
        created_private: bool,
    ) -> anyhow::Result<ModifyParams<'a>>
    where
        S: fmt::Display,
    {
        let mut params = client.service().modify_params();

        if let Some(values) = self.alias {
            params.alias(values);
        }

        if let Some(value) = self.assignee.as_ref() {
            if value.is_empty() {
                params.assignee(None);
            } else {
                params.assignee(Some(value));
            }
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
                value = edit_comment(value.trim())?;
            }
            params.comment(&value, created_private);
        }

        if let Some(value) = self.component {
            params.component(value);
        }

        if let Some(values) = self.custom_fields {
            params.custom_fields(values);
        }

        if let Some(values) = self.depends {
            params.depends(values);
        }

        if let Some(value) = self.duplicate_of {
            params.duplicate_of(value);
        }

        if let Some(values) = self.flags {
            params.flags(values);
        }

        if let Some(values) = self.groups {
            params.groups(values);
        }

        if let Some(values) = self.keywords {
            params.keywords(values);
        }

        if let Some(value) = self.os {
            params.os(value);
        }

        if let Some(value) = self.platform {
            params.platform(value);
        }

        if let Some(value) = self.priority {
            params.priority(value);
        }

        if let Some(value) = self.private_comment.as_ref() {
            if let Some(container) = value.container.as_ref() {
                let id = match ids {
                    [x] => x,
                    _ => anyhow::bail!("can't toggle comment privacy for multiple bugs"),
                };
                let comments = client
                    .comment(&[id], None)
                    .await?
                    .into_iter()
                    .next()
                    .expect("invalid comments response");

                let mut toggled = vec![];
                for c in comments {
                    if container.contains(&c.count) {
                        toggled.push((c.id, value.is_private.unwrap_or(!c.is_private)));
                    }
                }

                params.comment_is_private(toggled);
            }
        }

        if let Some(value) = self.product {
            params.product(value);
        }

        if let Some(value) = self.qa.as_ref() {
            if value.is_empty() {
                params.qa(None);
            } else {
                params.qa(Some(value));
            }
        }

        if let Some(value) = self.resolution {
            params.resolution(value);
        }

        if let Some(values) = self.see_also {
            params.see_also(values);
        }

        if let Some(value) = self.severity {
            params.severity(value);
        }

        if let Some(value) = self.status {
            params.status(value);
        }

        if let Some(value) = self.target {
            params.target(value);
        }

        if let Some(value) = self.summary {
            params.summary(value);
        }

        if let Some(value) = self.url {
            params.url(value);
        }

        if let Some(value) = self.version {
            params.version(value);
        }

        if let Some(value) = self.whiteboard {
            params.whiteboard(value);
        }

        Ok(params)
    }
}

impl From<Options> for Attributes {
    fn from(value: Options) -> Self {
        Self {
            alias: value.alias,
            assignee: value.assignee,
            blocks: value.blocks,
            cc: value.cc,
            comment: value.comment,
            component: value.component,
            depends: value.depends,
            duplicate_of: value.duplicate_of,
            flags: value.flags,
            groups: value.groups,
            keywords: value.keywords,
            os: value.os,
            platform: value.platform,
            priority: value.priority,
            private_comment: value.private_comment,
            product: value.product,
            qa: value.qa,
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
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        conflicts_with = "comment",
        help_heading = "Modify options",
        long_help = wrapped_doc!("
            Interactively reply to specific comments for a given bug.

            Values must be valid comment IDs specific to the bug, starting at 0
            for the description. If no value is specified the last comment will
            be used.

            This option forces interactive usage, launching an editor
            pre-populated with the selected comments allowing the user to
            respond in a style reminiscent of threaded messages on a mailing
            list. On completion, the data is used to create a new bug comment.

            Multiple arguments can be specified in a comma-separated list.

            Examples modifying bug 123:
            - reply to comments 1 and 2
            > bite m 123 --reply 1,2

            - reply to the last comment
            > bite m 123 --reply

            - private reply to last comment
            > bite m 123 --reply --private-comment
        ")
    )]
    reply: Option<Vec<usize>>,

    /// read attributes from a template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Read modification attributes from a template.

            Value must be the path to a valid modify template file. Templates
            use the TOML format and generally map long option names to values.

            Fields that don't match known bug field names are used for custom
            field modifications.
        ")
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to a template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Write modification attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating modify
            templates without any service interaction.
        ")
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    options: Options,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs or aliases
    #[clap(
        help_heading = "Arguments",
        required_unless_present = "dry_run",
        long_help = wrapped_doc!("
            IDs of bugs to modify.

            Taken from standard input when `-`.
        ")
    )]
    ids: Vec<MaybeStdinVec<String>>,
}

/// Interactively create a reply, pulling specified comments for pre-population.
async fn get_reply(
    client: &Client,
    id: &str,
    comment_ids: &mut Vec<usize>,
) -> anyhow::Result<String> {
    let comments = client
        .comment(&[id], None)
        .await?
        .into_iter()
        .next()
        .expect("invalid comments response");
    if comments.is_empty() {
        anyhow::bail!("reply invalid, bug {id} has no comments")
    }

    // use the last comment if no IDs were specified
    if comment_ids.is_empty() {
        comment_ids.push(comments.len() - 1);
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
    edit_comment(&data)
}

/// Interactively edit a comment.
fn edit_comment(data: &str) -> anyhow::Result<String> {
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
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
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

        if !self.dry_run {
            let ids = &self.ids.iter().flatten().collect::<Vec<_>>();
            let created_private = attrs
                .private_comment
                .as_ref()
                .map(|x| x.created_private())
                .unwrap_or_default();
            let mut params = attrs.into_params(client, ids, created_private).await?;

            // interactively create a reply
            if let Some(mut values) = self.reply {
                if ids.len() > 1 {
                    anyhow::bail!("reply invalid, targeting multiple bugs");
                }
                let comment = get_reply(client, ids[0], &mut values).await?;
                params.comment(comment.trim(), created_private);
            }

            let changes = client.modify(ids, params).await?;
            for change in changes {
                info!("{change}");
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn examples() {
        subcmd_parse_examples(&["bugzilla", "modify"]);
    }
}
