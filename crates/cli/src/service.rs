use bugbite::service::ClientParameters;
use camino::Utf8PathBuf;
use clap::Args;

// service modules
pub(crate) mod bugzilla;
pub(crate) mod github;
pub(crate) mod redmine;

#[derive(Args, Debug)]
#[clap(next_help_heading = "Service options")]
struct ServiceOptions {
    /// service connection
    #[arg(short, long, env = "BUGBITE_CONNECTION")]
    connection: Option<String>,

    /// concurrent requests limit
    #[arg(short = 'C', long, value_name = "VALUE")]
    concurrent: Option<usize>,

    /// custom root certificate
    #[arg(long, value_name = "PATH", conflicts_with = "insecure")]
    certificate: Option<Utf8PathBuf>,

    /// ignore invalid service certificates
    #[arg(
        short = 'I',
        long,
        num_args = 0,
        default_missing_value = "true",
        conflicts_with = "certificate"
    )]
    insecure: Option<bool>,

    /// custom proxy
    #[arg(short = 'P', long, value_name = "URL")]
    proxy: Option<String>,

    /// request timeout in seconds
    #[arg(short, long, value_name = "SECONDS")]
    timeout: Option<f64>,
}

impl From<ServiceOptions> for ClientParameters {
    fn from(value: ServiceOptions) -> Self {
        Self {
            certificate: value.certificate,
            concurrent: value.concurrent,
            insecure: value.insecure,
            proxy: value.proxy,
            timeout: value.timeout,
        }
    }
}

#[derive(Args, Debug)]
#[clap(next_help_heading = "Template options")]
pub(super) struct TemplateOptions {
    /// skip service interaction
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// read attributes from templates
    #[arg(long, value_name = "NAME[,...]", value_delimiter = ',')]
    from: Option<Vec<String>>,

    /// write attributes to template
    #[arg(long, value_name = "NAME")]
    to: Option<String>,
}
