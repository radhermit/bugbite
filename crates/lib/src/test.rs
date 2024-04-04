use std::fs;

use camino::Utf8Path;
use wiremock::{matchers, Match, Mock, MockServer, ResponseTemplate};

use crate::args::maybe_stdin::STDIN_HAS_BEEN_USED;
use crate::client::Client;
use crate::service::ServiceKind;

/// Reset standard input argument usage flag.
pub fn reset_stdin() {
    STDIN_HAS_BEEN_USED.store(false, std::sync::atomic::Ordering::SeqCst);
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

    pub fn client(&self, kind: ServiceKind) -> Client {
        Client::new(kind, self.uri()).unwrap()
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
