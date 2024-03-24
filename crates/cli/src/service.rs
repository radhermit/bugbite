// args, output, and rendering support
pub(crate) mod args;
pub(crate) mod output;

// service modules
pub(crate) mod bugzilla;
pub(crate) mod github;
pub(crate) mod redmine;

/// Render an object for output to the terminal.
pub(crate) trait Render {
    fn render<W: std::io::Write>(&self, f: &mut W, width: usize) -> std::io::Result<()>;
}
