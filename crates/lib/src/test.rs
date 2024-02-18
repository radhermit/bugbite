use camino::Utf8PathBuf;
use once_cell::sync::Lazy;

/// Build a [`Utf8PathBuf`] path from a base and components.
#[macro_export]
macro_rules! build_path {
    ($base:expr, $($segment:expr),+) => {{
        let mut base: ::camino::Utf8PathBuf = $base.into();
        $(base.push($segment);)*
        base
    }}
}
pub(crate) use build_path;

pub(crate) static TESTDATA_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));
