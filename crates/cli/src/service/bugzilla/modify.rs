use std::hash::Hash;
use std::process::ExitCode;
use std::str::FromStr;
use std::{fmt, fs};

use anyhow::Context;
use bugbite::args::{MaybeStdin, MaybeStdinVec};
use bugbite::client::bugzilla::Client;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::modify::{Parameters, RangeOrSet, SetChange};
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use tempfile::NamedTempFile;
use tracing::info;

use crate::utils::{confirm, launch_editor, wrapped_doc};

#[derive(Debug, Clone)]
struct CommentPrivacy<T: FromStr + PartialOrd + Eq + Hash> {
    raw: String,
    range_or_set: Option<RangeOrSet<T>>,
    is_private: Option<bool>,
}

impl<T: FromStr + PartialOrd + Eq + Hash> FromStr for CommentPrivacy<T>
where
    <T as FromStr>::Err: fmt::Display + fmt::Debug,
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (range_or_set, is_private) = if let Some((ids, value)) = s.split_once(':') {
            (Some(ids.parse()?), Some(value.parse()?))
        } else {
            (Some(s.parse()?), None)
        };

        Ok(Self {
            raw: s.to_string(),
            range_or_set,
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

            - add yourself to the CC list
            > bite m 10 --cc @me

            Prefixing values with `+` or `-` adds or removes from the list,
            respectively. Unprefixed values will be added to the list.

            - remove yourself from the CC list
            > bite m 10 --cc=-@me

            Multiple arguments can be specified in a comma-separated list.

            - add and remove addresses from the CC list
            > bite m 10 --cc=+test1@email.com,-test2@email.com
        ")
    )]
    cc: Option<Vec<SetChange<String>>>,

    /// add a comment
    #[arg(
        short = 'c',
        long,
        num_args = 0..=1,
        conflicts_with_all = ["comment_from", "reply"],
        default_missing_value = "",
        long_help = wrapped_doc!("
            Add a comment.

            When no argument is specified, an editor is launched allowing for
            interactive entry.

            Taken from standard input when `-`.
        ")
    )]
    comment: Option<MaybeStdin<String>>,

    /// load comment from file
    #[arg(
        short = 'F',
        long,
        conflicts_with_all = ["comment", "reply"],
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Add a comment using content from a file.

            The value must be the path to a valid comment file.

            Example modifying bug 10:
            - create a comment from a file
            > bite m 10 --comment-from path/to/file.txt
        ")
    )]
    comment_from: Option<Utf8PathBuf>,

    /// enable comment privacy
    #[arg(
        short = 'P',
        long,
        num_args = 0,
        default_missing_value = "true",
        long_help = wrapped_doc!("
            Mark created comment as private.

            Examples modifying bug 10:
            - create a private comment
            > bite m 10 --comment test --comment-is-private

            - create a private comment from a file
            > bite m 10 --comment-from path/to/file.txt --comment-is-private

            - private reply to last comment
            > bite m 10 --reply --comment-is-private
        ")
    )]
    comment_is_private: Option<bool>,

    /// modify comment privacy
    #[arg(
        long,
        value_name = "VALUE",
        long_help = wrapped_doc!("
            Modify the privacy of existing comments.

            The value must be comma-separated comment IDs local to the specified
            bug ID starting at 0 for the bug description or a range of comment
            IDs. An optional suffix consisting of boolean value in the form of
            `:true` or `:false` can be included to enable or disable all comment
            privacy respectively. Without this suffix, the privacy of all
            matching comments is toggled.

            Examples modifying bug 10:
            - toggle comment 1 privacy
            > bite m 10 --comment-privacy 1

            - toggle comment 1 and 2 privacy
            > bite m 10 --comment-privacy 1,2

            - toggle all comment privacy
            > bite m 10 --comment-privacy ..

            - disable comment 1 and 2 privacy
            > bite m 10 --comment-privacy 1,2:false

            - mark comments 2-5 private
            > bite m 10 --comment-privacy 2..=5:true
        ")
    )]
    comment_privacy: Option<CommentPrivacy<usize>>,

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
        short,
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

impl From<Options> for Parameters {
    fn from(value: Options) -> Self {
        Self {
            alias: value.alias,
            assignee: value.assignee,
            blocks: value.blocks,
            cc: value.cc,
            comment: value.comment.map(|x| x.into_inner()),
            comment_from: value.comment_from,
            comment_is_private: value.comment_is_private,
            comment_privacy: value
                .comment_privacy
                .and_then(|x| x.range_or_set.map(|value| (value, x.is_private))),
            component: value.component,
            depends: value.depends,
            duplicate_of: value.duplicate_of,
            flags: value.flags,
            groups: value.groups,
            keywords: value.keywords,
            os: value.os,
            platform: value.platform,
            priority: value.priority,
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
        conflicts_with_all = ["comment", "comment_from"],
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
        ")
    )]
    reply: Option<Vec<usize>>,

    /// read attributes from template
    #[arg(
        long,
        help_heading = "Modify options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Read modification attributes from a template.

            Value must be the path to a valid template file. Templates use the
            TOML format and generally map long option names to values.

            Fields that don't match known bug field names target custom fields.

            Explicitly specified options override corresponding template values.
        ")
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to template
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
        fs::write(&temp_file, data).context("failed saving comment file")?;
    }

    loop {
        let status = launch_editor(&temp_file)?;
        if !status.success() {
            anyhow::bail!("failed editing reply content");
        }
        let comment = fs::read_to_string(&temp_file).context("failed reading comment file")?;
        if comment != data || confirm("No changes made to comment, submit anyway?", false)? {
            return Ok(comment.trim().to_string());
        }
    }
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let ids = &self.ids.iter().flatten().collect::<Vec<_>>();

        let mut params: Parameters = self.options.into();

        // read modification attributes from template
        if let Some(path) = self.from.as_ref() {
            let template = Parameters::from_path(path)?;
            // command-line options override template options
            params = params.merge(template);
        };

        // write modification attributes to template
        if let Some(path) = self.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&params)?;
                fs::write(path, data).context("failed writing template")?;
            }
        }

        // interactively create reply or comment
        if let Some(mut values) = self.reply {
            if ids.len() > 1 {
                anyhow::bail!("reply invalid, targeting multiple bugs");
            }
            let comment = get_reply(client, ids[0], &mut values).await?;
            params.comment = Some(comment);
        } else if let Some(path) = params.comment_from.take() {
            let comment =
                fs::read_to_string(path).context("failed reading comment file: {path}")?;
            params.comment = Some(comment);
        } else if let Some(value) = params.comment.as_ref() {
            if value.trim().is_empty() {
                let comment = edit_comment(value.trim())?;
                params.comment = Some(comment);
            }
        }

        if !self.dry_run {
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
