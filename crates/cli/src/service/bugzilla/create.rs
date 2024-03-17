use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::create::CreateParams;
use bugbite::traits::WebClient;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use tracing::info;

use crate::macros::async_block;
use crate::utils::confirm;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// set alias
    #[arg(short = 'A', long)]
    alias: Option<String>,

    /// assign to a user
    #[arg(
        short,
        long,
        value_name = "USER",
        long_help = indoc::indoc! {"
            Assign a bug to a user.

            The value must be an email address for a service user. The alias
            `@me` can also be used for the service's configured user if one
            exists.
        "}
    )]
    assigned_to: Option<String>,

    /// set blockers
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set blockers.

            Values must be valid IDs for existing bugs.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.
        "}
    )]
    blocks: Option<Vec<NonZeroU64>>,

    /// set CC users
    #[arg(
        long,
        value_name = "USER[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set users in the CC list.

            Values must be email addresses for service users.

            Multiple arguments can be specified in a comma-separated list.
        "}
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
        num_args = 0..=1,
        value_name = "ID[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set dependencies.

            Values must be valid IDs for existing bugs.

            Multiple arguments can be specified in a comma-separated list while
            no arguments removes the entire list.
        "}
    )]
    depends_on: Option<Vec<NonZeroU64>>,

    /// set description
    #[arg(short = 'D', long)]
    description: Option<String>,

    /// set groups
    #[arg(
        short,
        long,
        value_name = "GROUP[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set groups.

            Values must be valid service groups.

            Multiple arguments can be specified in a comma-separated list.
        "}
    )]
    groups: Option<Vec<String>>,

    /// set keywords
    #[arg(
        short,
        long,
        num_args = 0..=1,
        value_name = "KW[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set keywords.

            Values must be valid keywords.

            Multiple arguments can be specified in a comma-separated list.
        "}
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

    /// set resolution
    #[arg(short, long)]
    resolution: Option<String>,

    /// set external bug URLs
    #[arg(
        short = 'U',
        long,
        value_name = "URL[,...]",
        value_delimiter = ',',
        long_help = indoc::indoc! {"
            Set URLs to bugs in external trackers.

            Values must be valid URLs to bugs, issues, or tickets in external
            trackers.

            Multiple arguments can be specified in a comma-separated list.
        "}
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
    #[arg(short, long, value_name = "MILESTONE")]
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
    alias: Option<String>,
    assigned_to: Option<String>,
    blocks: Option<Vec<NonZeroU64>>,
    cc: Option<Vec<String>>,
    component: Option<String>,
    depends_on: Option<Vec<NonZeroU64>>,
    description: Option<String>,
    groups: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    os: Option<String>,
    platform: Option<String>,
    priority: Option<String>,
    product: Option<String>,
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
            assigned_to: self.assigned_to.or(other.assigned_to),
            blocks: self.blocks.or(other.blocks),
            cc: self.cc.or(other.cc),
            component: self.component.or(other.component),
            depends_on: self.depends_on.or(other.depends_on),
            description: self.description.or(other.description),
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

    fn into_params(self, client: &Client) -> CreateParams {
        let mut params = client.service().create_params();

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

        if let Some(value) = self.component.as_ref() {
            params.component(value);
        }

        if let Some(values) = self.custom_fields {
            params.custom_fields(values);
        }

        if let Some(values) = self.depends_on {
            params.depends_on(values);
        }

        if let Some(value) = self.description.as_ref() {
            params.description(value);
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

        params
    }
}

impl From<Options> for Attributes {
    fn from(value: Options) -> Self {
        Self {
            alias: value.alias,
            assigned_to: value.assigned_to,
            blocks: value.blocks,
            cc: value.cc,
            component: value.component,
            depends_on: value.depends_on,
            description: value.description,
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
    #[arg(short = 'n', long, help_heading = "Create options")]
    dry_run: bool,

    /// read attributes from a template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = indoc::indoc! {"
            Read attributes from a template.

            Value must be the path to a valid template file. Templates
            use the TOML format and generally map long option names to values.

            Fields that don't match known bug field names are used for custom
            fields.
        "}
    )]
    from: Option<Utf8PathBuf>,

    /// write attributes to a template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = indoc::indoc! {"
            Write attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating
            templates without any service interaction.
        "}
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    options: Options,
}

impl Command {
    pub(super) fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut attrs: Attributes = self.options.into();

        // read attributes from a template
        if let Some(path) = self.from.as_ref() {
            let data = fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("failed loading template: {path}: {e}"))?;
            let template = toml::from_str(&data)
                .map_err(|e| anyhow::anyhow!("failed parsing template: {path}: {e}"))?;
            // command-line options override template options
            attrs = attrs.merge(template);
        };

        // write attributes to a template
        if let Some(path) = self.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&attrs)?;
                fs::write(path, data)?;
            }
        }

        let params = attrs.into_params(client);

        if !self.dry_run {
            let id = async_block!(client.create(params))?;
            info!("Created bug #{id}");
        }

        Ok(ExitCode::SUCCESS)
    }
}
