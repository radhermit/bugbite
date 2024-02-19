use std::fs;

use camino::Utf8PathBuf;
use once_cell::sync::Lazy;
use wiremock::{Match, Mock, MockServer, ResponseTemplate};

use crate::client::Client;
use crate::service::ServiceKind;

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

pub(crate) struct TestServer {
    server: MockServer,
    uri: String,
}

impl TestServer {
    pub(crate) async fn new() -> Self {
        let server = MockServer::start().await;
        let uri = server.uri();
        Self { server, uri }
    }

    pub(crate) fn client(&self, kind: ServiceKind) -> Client {
        let service = kind.create(self.uri()).unwrap();
        Client::builder().build(service).unwrap()
    }

    pub(crate) fn server(&self) -> &MockServer {
        &self.server
    }

    pub(crate) fn uri(&self) -> &str {
        &self.uri
    }

    pub(crate) async fn respond<M: 'static + Match>(&self, matcher: M, status: u16, path: &str) {
        let json = fs::read_to_string(TESTDATA_PATH.join(path)).unwrap();
        let template =
            ResponseTemplate::new(status).set_body_raw(json.as_bytes(), "application/json");
        Mock::given(matcher)
            .respond_with(template)
            .mount(self.server())
            .await;
    }

    pub(crate) async fn reset(&self) {
        self.server().reset().await;
    }
}
