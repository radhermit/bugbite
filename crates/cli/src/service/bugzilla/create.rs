use std::collections::HashMap;
use std::fs;
use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::objects::bugzilla::{Bug, Flag};
use bugbite::service::bugzilla::create::CreateParams;
use bugbite::traits::WebClient;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::info;

use crate::utils::{confirm, wrapped_doc};

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// set aliases
    #[arg(short = 'A', long, value_name = "VALUE[,...]", value_delimiter = ',')]
    alias: Option<Vec<String>>,

    /// set assignee
    #[arg(
        short,
        long,
        value_name = "USER",
        long_help = wrapped_doc!("
            Assign a bug to a user.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.
        ")
    )]
    assignee: Option<String>,

    /// set blockers
    #[arg(
        short,
        long,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set blockers.

            Values must be valid IDs for existing bugs.

            Multiple arguments can be specified in a comma-separated list or are
            taken from standard input when `-`.
        ")
    )]
    blocks: Option<Vec<MaybeStdinVec<u64>>>,

    /// set CC users
    #[arg(
        long,
        value_name = "USER[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set users in the CC list.

            Values must be email addresses for service users.

            Multiple arguments can be specified in a comma-separated list.
        ")
    )]
    cc: Option<Vec<String>>,

    /// set component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// set custom field
    #[arg(long = "cf", num_args = 2, value_names = ["NAME", "VALUE"])]
    custom_fields: Option<Vec<String>>,

    /// set dependencies
    #[arg(
        short,
        long,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set dependencies.

            Values must be valid IDs for existing bugs.

            Multiple arguments can be specified in a comma-separated list or are
            taken from standard input when `-`.
        ")
    )]
    depends: Option<Vec<MaybeStdinVec<u64>>>,

    /// set description
    #[arg(short = 'D', long)]
    description: Option<String>,

    /// set flags
    #[arg(
        short = 'F',
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set flags.

            Values must be valid flags composed of the flag name followed by its
            status. Supported statuses include `+`, `-`, and `?`.

            Multiple arguments can be specified in a comma-separated list.
        ")
    )]
    flags: Option<Vec<Flag>>,

    /// set groups
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set groups.

            Values must be valid service groups. No arguments may be used to
            avoid adding the bug to all default groups for the targeted product.

            Multiple arguments can be specified in a comma-separated list.
        ")
    )]
    groups: Option<Vec<String>>,

    /// set keywords
    #[arg(
        short,
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set keywords.

            Values must be valid keywords.

            Multiple arguments can be specified in a comma-separated list.
        ")
    )]
    keywords: Option<Vec<String>>,

    /// set operating system
    #[arg(long)]
    os: Option<String>,

    /// set platform
    #[arg(long)]
    platform: Option<String>,

    /// set priority
    #[arg(long)]
    priority: Option<String>,

    /// set product
    #[arg(short, long)]
    product: Option<String>,

    /// set QA contact
    #[arg(
        long,
        value_name = "USER",
        long_help = wrapped_doc!("
            Set the QA contact for a bug.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.
        ")
    )]
    qa: Option<String>,

    /// set resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// set external bug URLs
    #[arg(
        short = 'U',
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set URLs to bugs in external trackers.

            Values must be valid URLs to bugs, issues, or tickets in external
            trackers.

            Multiple arguments can be specified in a comma-separated list.
        ")
    )]
    see_also: Option<Vec<String>>,

    /// set severity
    #[arg(long)]
    severity: Option<String>,

    /// set status
    #[arg(short, long)]
    status: Option<String>,

    /// set summary
    #[arg(short = 'S', long)]
    summary: Option<String>,

    /// set target milestone
    #[arg(short = 'T', long, value_name = "MILESTONE")]
    target: Option<String>,

    /// set URL
    #[arg(short = 'u', long)]
    url: Option<String>,

    /// set version
    #[arg(short = 'V', long)]
    version: Option<String>,

    /// set whiteboard
    #[arg(short, long)]
    whiteboard: Option<String>,
}

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct Attributes {
    alias: Option<Vec<String>>,
    assignee: Option<String>,
    blocks: Option<Vec<u64>>,
    cc: Option<Vec<String>>,
    component: Option<String>,
    depends: Option<Vec<u64>>,
    description: Option<String>,
    flags: Option<Vec<Flag>>,
    groups: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    os: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    product: Option<String>,
    qa: Option<String>,
    resolution: Option<String>,
    see_also: Option<Vec<String>>,
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
            component: self.component.or(other.component),
            depends: self.depends.or(other.depends),
            description: self.description.or(other.description),
            flags: self.flags.or(other.flags),
            groups: self.groups.or(other.groups),
            keywords: self.keywords.or(other.keywords),
            os: self.os.or(other.os),
            platform: self.platform.or(other.platform),
            priority: self.priority.or(other.priority),
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

    fn into_params(self, client: &Client) -> CreateParams {
        let mut params = client.service().create_params();

        if let Some(values) = self.alias {
            params.alias(values);
        }

        if let Some(value) = self.assignee.as_ref() {
            params.assignee(value);
        }

        if let Some(values) = self.blocks {
            params.blocks(values);
        }

        if let Some(values) = self.cc {
            params.cc(values);
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

        if let Some(value) = self.description {
            params.description(value);
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

        if let Some(value) = self.product {
            params.product(value);
        }

        if let Some(value) = self.qa.as_ref() {
            params.qa(value);
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

        params
    }
}

impl From<Options> for Attributes {
    fn from(value: Options) -> Self {
        Self {
            alias: value.alias,
            assignee: value.assignee,
            blocks: value.blocks.map(|x| x.into_iter().flatten().collect()),
            cc: value.cc,
            component: value.component,
            depends: value.depends.map(|x| x.into_iter().flatten().collect()),
            description: value.description,
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

impl From<Bug> for Attributes {
    fn from(value: Bug) -> Self {
        Self {
            component: value.component,
            os: value.op_sys,
            platform: value.platform,
            priority: value.priority,
            product: value.product,
            severity: value.severity,
            version: value.version,
            ..Default::default()
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct Command {
    /// skip service interaction
    #[arg(short = 'n', long, help_heading = "Create options")]
    dry_run: bool,

    /// read attributes from a template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        conflicts_with = "from_bug",
        long_help = wrapped_doc!("
            Read attributes from a template.

            Value must be the path to a valid template file. Templates
            use the TOML format and generally map long option names to values.

            Fields that don't match known bug field names are used for custom
            fields.
        ")
    )]
    from: Option<Utf8PathBuf>,

    /// read attributes from an existing bug
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "ID",
        conflicts_with = "from",
        long_help = wrapped_doc!("
            Read attributes from an existing bug.

            Value must be the ID of an existing bug which will be used to
            pre-populate the relevant, required fields for creation.

            Combining this option with -n/--dry-run and --to allows creating
            templates using existing bugs to edit and use later without creating
            a new bug.
        ")
    )]
    from_bug: Option<u64>,

    /// write attributes to a template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Write attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating
            templates without any service interaction.
        ")
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    options: Options,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut attrs: Attributes = self.options.into();

        // read attributes from a template
        if let Some(path) = self.from.as_ref() {
            let data = fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("failed loading template: {path}: {e}"))?;
            let template = toml::from_str(&data)
                .map_err(|e| anyhow::anyhow!("failed parsing template: {path}: {e}"))?;
            // command-line options override template options
            attrs = attrs.merge(template);
        } else if let Some(id) = self.from_bug {
            let bug = client
                .get(&[id], false, false, false)
                .await?
                .into_iter()
                .next()
                .expect("failed getting bug");
            attrs = attrs.merge(bug.into());
        }

        // write attributes to a template
        if let Some(path) = self.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&attrs)?;
                fs::write(path, data)?;
            }
        }

        let params = attrs.into_params(client);

        if !self.dry_run {
            let mut stdout = stdout().lock();
            let id = client.create(params).await?;
            if stdout.is_terminal() {
                info!("Created bug {id}");
            } else {
                writeln!(stdout, "{id}")?;
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
        subcmd_parse_examples(&["bugzilla", "create"]);
    }
}
