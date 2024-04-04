use std::fs;
use std::io::{stdout, IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::objects::bugzilla::Flag;
use bugbite::service::bugzilla::create::Parameters;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};
use itertools::Itertools;
use tracing::info;

use crate::utils::{confirm, wrapped_doc};

#[derive(Args, Debug)]
#[clap(next_help_heading = "Attribute options")]
struct Options {
    /// set aliases
    #[arg(
        short = 'A',
        long,
        value_name = "VALUE[,...]",
        value_delimiter = ',',
        long_help = wrapped_doc!("
            Set aliases.

            The values must be unique aliases for a bug, using existing aliases
            will cause the service to return an error.

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
        ")
    )]
    alias: Option<Vec<String>>,

    /// set assignee
    #[arg(
        short,
        long,
        value_name = "USER",
        long_help = wrapped_doc!("
            Assign a bug to a user.

            The value must be an email address for a service user. The alias
            `@me` can be used for the service's configured user if one exists.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.

            Values are taken from standard input when `-`.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
        ")
    )]
    cc: Option<Vec<String>>,

    /// set component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// set custom field
    #[arg(
        long = "cf",
        num_args = 2,
        value_names = ["NAME", "VALUE"],
        long_help = wrapped_doc!("
            Set custom fields.

            The values must be valid custom field names followed by their value.

            Multiple arguments can be specified via multiple options.
        ")
    )]
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.

            Values are taken from standard input when `-`.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
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

            Multiple arguments can be specified in a comma-separated list or via
            multiple options.
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

impl From<Options> for Parameters {
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

#[derive(Debug, Args)]
pub(super) struct Command {
    /// skip service interaction
    #[arg(short = 'n', long, help_heading = "Create options")]
    dry_run: bool,

    /// read attributes from template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        conflicts_with = "from_bug",
        long_help = wrapped_doc!(r#"
            Read attributes from a template.

            Value must be the path to a valid template file. Templates use the
            TOML format and generally map long option names to values.

            Fields that don't match known bug field names target custom fields.

            Explicitly specified options override corresponding template values.

            Example:
            - create bug using template
            > bite c --from path/to/new.toml -S "summary" -D "description"
        "#)
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

            Example:
            - create template using existing bug
            > bite c --from-bug 123 --to path/to/new.toml --dry-run
        ")
    )]
    from_bug: Option<u64>,

    /// write attributes to template
    #[arg(
        long,
        help_heading = "Create options",
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        long_help = wrapped_doc!("
            Write attributes to a template.

            Value is the file path where the TOML template file will be written.

            Combining this option with -n/--dry-run allows creating templates
            without any service interaction.

            Example:
            - create template using specified values
            > bite c -p TestProduct -C TestComponent --to path/to/new.toml --dry-run
        ")
    )]
    to: Option<Utf8PathBuf>,

    #[clap(flatten)]
    options: Options,
}

impl Command {
    pub(super) async fn run(self, client: &Client) -> anyhow::Result<ExitCode> {
        let mut params: Parameters = self.options.into();

        // read attributes from template
        if let Some(path) = self.from.as_ref() {
            let template = Parameters::from_path(path)?;
            // command-line options override template options
            params = params.merge(template);
        } else if let Some(id) = self.from_bug {
            let bug = client
                .get(&[id], false, false, false)
                .await?
                .into_iter()
                .next()
                .expect("failed getting bug");
            params = params.merge(bug.into());
        }

        // write attributes to template
        if let Some(path) = self.to.as_ref() {
            if !path.exists() || confirm(format!("template exists: {path}, overwrite?"), false)? {
                let data = toml::to_string(&params)?;
                fs::write(path, data)?;
            }
        }

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
