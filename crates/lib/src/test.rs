use std::fs;

use camino::Utf8Path;
use wiremock::{matchers, Match, Mock, MockServer, ResponseTemplate};

use crate::args::maybe_stdin::STDIN_HAS_BEEN_USED;

/// Reset standard input argument usage flag.
pub fn reset_stdin() {
    STDIN_HAS_BEEN_USED.store(false, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) static TESTDATA_PATH: once_cell::sync::Lazy<camino::Utf8PathBuf> =
    once_cell::sync::Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

pub struct TestServer {
    server: MockServer,
    uri: String,
}

impl TestServer {
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let uri = server.uri();
        Self { server, uri }
    }

    pub fn mock(&self) -> &MockServer {
        &self.server
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub async fn respond_match<M, P>(&self, matcher: M, status: u16, path: P)
    where
        M: 'static + Match,
        P: AsRef<Utf8Path>,
    {
        let path = path.as_ref();
        let json = fs::read_to_string(path).unwrap_or_else(|e| panic!("invalid path: {path}: {e}"));
        let template =
            ResponseTemplate::new(status).set_body_raw(json.as_bytes(), "application/json");
        Mock::given(matcher)
            .respond_with(template)
            .mount(&self.server)
            .await;
    }

    pub async fn respond<P: AsRef<Utf8Path>>(&self, status: u16, path: P) {
        self.respond_match(matchers::any(), status, path).await
    }

    pub async fn respond_custom<M>(&self, matcher: M, response: ResponseTemplate)
    where
        M: 'static + Match,
    {
        Mock::given(matcher)
            .respond_with(response)
            .mount(&self.server)
            .await;
    }

    pub async fn reset(&self) {
        self.server.reset().await;
    }
}

/// Build a [`Utf8PathBuf`] path from a base and components.
#[macro_export]
macro_rules! build_path {
    ($base:expr, $($segment:expr),+) => {{
        let mut base: ::camino::Utf8PathBuf = $base.into();
        $(base.push($segment);)*
        base
    }}
}
pub use build_path;

/// Verify two, ordered iterables are equal.
#[macro_export]
macro_rules! assert_ordered_eq {
    ($iter1:expr, $iter2:expr, $msg:expr) => {{
        let a: Vec<_> = $iter1.into_iter().collect();
        let b: Vec<_> = $iter2.into_iter().collect();
        let msg = $msg;
        assert_eq!(a, b, "{msg}");
    }};

    ($iter1:expr, $iter2:expr $(,)?) => {{
        assert_ordered_eq!($iter1, $iter2, "");
    }};
}
pub use assert_ordered_eq;

/// Verify two, unordered iterables contain the same elements.
#[macro_export]
macro_rules! assert_unordered_eq {
    ($iter1:expr, $iter2:expr, $msg:expr) => {{
        let mut a: Vec<_> = $iter1.into_iter().collect();
        let mut b: Vec<_> = $iter2.into_iter().collect();
        a.sort();
        b.sort();
        let msg = $msg;
        assert_eq!(a, b, "{msg}");
    }};

    ($iter1:expr, $iter2:expr $(,)?) => {{
        assert_unordered_eq!($iter1, $iter2, "");
    }};
}
pub use assert_unordered_eq;

/// Assert an error matches a given regular expression for testing.
#[macro_export]
macro_rules! assert_err_re {
    ($err:expr, $x:expr) => {
        $crate::test::assert_err_re!($err, $x, "");
    };
    ($err:expr, $re:expr, $msg:expr) => {
        let s = $err.to_string();
        let re = ::regex::Regex::new($re.as_ref()).unwrap();
        let err_msg = format!("{s:?} does not match regex: {:?}", $re);
        if $msg.is_empty() {
            assert!(re.is_match(&s), "{}", err_msg);
        } else {
            assert!(re.is_match(&s), "{}", format!("{err_msg}: {}", $msg));
        }
    };
}
pub use assert_err_re;
