use std::io::{self, IsTerminal, Write};

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

#[derive(clap::Args)]
#[clap(next_help_heading = "Service options")]
struct ServiceOptions {
    /// service connection
    #[arg(short, long, env = "BUGBITE_CONNECTION")]
    connection: String,

    /// ignore invalid service certificates
    #[arg(long)]
    insecure: bool,

    /// request timeout in seconds
    #[arg(short, long, value_name = "SECONDS", default_value = "30")]
    timeout: f64,
}
