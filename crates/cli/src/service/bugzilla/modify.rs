use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::modify::ModifyParams;
use camino::Utf8PathBuf;
use clap::{Args, ValueHint};

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Modify options")]
struct Options {
    /// modify status
    #[arg(short, long)]
    status: Option<String>,

    /// modify resolution
    #[arg(short = 'R', long)]
    resolution: Option<String>,

    /// mark bug as duplicate
    #[arg(short, long, conflicts_with_all = ["status", "resolution"])]
    duplicate: Option<NonZeroU64>,

    /// modify component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// modify product
    #[arg(short = 'P', long)]
    product: Option<String>,

    /// add a comment
    #[arg(short = 'c', long)]
    comment: Option<String>,

    /// modify summary
    #[arg(short, long)]
    title: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

    /// load options from a template
    #[arg(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    template: Option<Utf8PathBuf>,

    // TODO: rework stdin support once clap supports custom containers
    // See: https://github.com/clap-rs/clap/issues/3114
    /// bug IDs
    #[clap(
        required = true,
        help_heading = "Arguments",
        long_help = indoc::indoc! {"
            IDs of bugs to modify.

            Taken from standard input when `-`.
        "}
    )]
    ids: Vec<MaybeStdinVec<NonZeroU64>>,
}

impl Command {
    pub(super) fn run(&self, client: &Client) -> Result<ExitCode, bugbite::Error> {
        let ids = &self.ids.iter().flatten().copied().collect::<Vec<_>>();

        let mut params = if let Some(path) = self.template.as_ref() {
            ModifyParams::load(path)?
        } else {
            ModifyParams::new()
        };

        if let Some(value) = self.options.status.as_ref() {
            params.status(value);
        }
        if let Some(value) = self.options.resolution.as_ref() {
            params.resolution(value);
        }
        if let Some(value) = self.options.duplicate {
            params.duplicate(value);
        }
        if let Some(value) = self.options.component.as_ref() {
            params.component(value);
        }
        if let Some(value) = self.options.product.as_ref() {
            params.product(value);
        }
        if let Some(value) = self.options.comment.as_ref() {
            params.comment(value);
        }
        if let Some(value) = self.options.title.as_ref() {
            params.summary(value);
        }

        async_block!(client.modify(ids, params))?;
        Ok(ExitCode::SUCCESS)
    }
}
