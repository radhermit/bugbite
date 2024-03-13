use std::num::NonZeroU64;
use std::process::ExitCode;

use bugbite::args::MaybeStdinVec;
use bugbite::client::bugzilla::Client;
use bugbite::service::bugzilla::modify::ModifyParams;
use clap::Args;

use crate::macros::async_block;

#[derive(Debug, Args)]
#[clap(next_help_heading = "Modify options")]
struct Options {
    /// set new status
    #[arg(short, long)]
    status: Option<String>,

    /// set new resolution
    #[arg(short = 'R', long)]
    resolution: Option<String>,

    /// set component
    #[arg(short = 'C', long)]
    component: Option<String>,

    /// set product
    #[arg(short = 'P', long)]
    product: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct Command {
    #[clap(flatten)]
    options: Options,

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
        let mut params = ModifyParams::new();
        if let Some(value) = self.options.status.as_ref() {
            params.status(value);
        }
        if let Some(value) = self.options.resolution.as_ref() {
            params.resolution(value);
        }
        if let Some(value) = self.options.component.as_ref() {
            params.component(value);
        }
        if let Some(value) = self.options.product.as_ref() {
            params.product(value);
        }

        async_block!(client.modify(ids, params))?;
        Ok(ExitCode::SUCCESS)
    }
}
