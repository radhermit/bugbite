use std::io::{self, IsTerminal, Write};

use bugbite::service::ClientParameters;
use camino::Utf8PathBuf;

// output and rendering support
pub(crate) mod output;

// service modules
pub(crate) mod bugzilla;
pub(crate) mod github;
pub(crate) mod redmine;

/// Render an item for output to the terminal.
pub(crate) trait Render<T> {
    fn render<W: IsTerminal + Write>(&self, item: T, f: &mut W, width: usize) -> io::Result<()>;
}

#[derive(clap::Args, Debug)]
#[clap(next_help_heading = "Service options")]
struct ServiceOptions {
    /// service connection
    #[arg(short, long, env = "BUGBITE_CONNECTION")]
    connection: String,

    /// add custom root certificate
    #[arg(long, value_name = "PATH", conflicts_with = "insecure")]
    certificate: Option<Utf8PathBuf>,

    /// ignore invalid service certificates
    #[arg(
        long,
        num_args = 0,
        default_missing_value = "true",
        conflicts_with = "certificate"
    )]
    insecure: Option<bool>,

    /// request timeout in seconds
    #[arg(short, long, value_name = "SECONDS")]
    timeout: Option<f64>,
}

impl From<ServiceOptions> for ClientParameters {
    fn from(value: ServiceOptions) -> Self {
        Self {
            certificate: value.certificate,
            insecure: value.insecure,
            timeout: value.timeout,
        }
    }
}
