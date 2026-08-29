use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use bugbite::args::{Csv, ExistsOrValues, MaybeStdinVec};
use bugbite::objects::RangeOrValue;
use bugbite::service::bugzilla::Bugzilla;
use bugbite::service::bugzilla::attachment::search::Parameters;
use bugbite::service::bugzilla::search::Match;
use bugbite::time::TimeDeltaOrStatic;
use bugbite::traits::{Merge, RequestSend, RequestTemplate};
use clap::Args;

use crate::service::TemplateOptions;

/// Available search parameters.
#[derive(Args, Debug)]
#[clap(next_help_heading = "Attachment options")]
struct Params {
    /// filter by creator
    #[arg(short = 'C', long)]
    creator: Option<Vec<Csv<Match>>>,

    /// filter by description
    #[arg(short, long)]
    description: Option<Vec<Csv<Match>>>,

    /// filter by file name
    #[arg(short, long)]
    filename: Option<Vec<Csv<Match>>>,

    /// filter by bug ID
    #[arg(short, long, num_args = 1, value_name = "ID[,...]")]
    id: Option<Vec<ExistsOrValues<MaybeStdinVec<RangeOrValue<i64>>>>>,

    /// filter by MIME type
    #[arg(short, long, value_name = "VALUE[,...]")]
    mime: Option<Vec<Csv<Match>>>,

    /// filter by size
    #[arg(short, long)]
    size: Option<RangeOrValue<u64>>,

    /// filter by creation time
    #[arg(short, long)]
    created: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// filter by update time
    #[arg(short, long, value_name = "TIME")]
    updated: Option<RangeOrValue<TimeDeltaOrStatic>>,

    /// filter by obsolete status
    #[arg(
        short = 'o',
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    obsolete: Option<bool>,

    /// filter by patch status
    #[arg(
        short = 'p',
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    patch: Option<bool>,

    /// filter by private status
    #[arg(
        short = 'P',
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_name = "BOOL",
        hide_possible_values = true,
    )]
    private: Option<bool>,
}

impl Merge<Params> for Parameters {
    fn merge(&mut self, other: Params) {
        self.merge(Self {
            creator: other
                .creator
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            description: other
                .description
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            filename: other
                .filename
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            ids: other
                .id
                .map(|x| x.into_iter().map(|x| x.flatten()).collect()),
            mime: other
                .mime
                .map(|x| x.into_iter().map(|x| x.into_inner()).collect()),
            size: other.size,

            created: other.created,
            updated: other.updated,

            obsolete: other.obsolete,
            patch: other.patch,
            private: other.private,
        })
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Search options")]
pub(super) struct Options {
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
        let mut request = service.attachment_search();

        // read attributes from templates
        if let Some(names) = &self.template.from {
            for name in names {
                request.load_template(name)?;
            }
        }

        // command line parameters override template
        request.params.merge(self.params);

        // write attributes to template
        if let Some(name) = &self.template.to {
            request.save_template(name)?;
        }

        if !self.template.dry_run {
            let attachments = request.send().await?;
            for attachment in attachments {
                write!(f, "{attachment}")?;
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}
