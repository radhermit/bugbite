use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use wiremock::{matchers, Match, Mock, MockServer, ResponseTemplate};

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
pub use build_path;

pub static TESTDATA_PATH: Lazy<Utf8PathBuf> =
    Lazy::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

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
        let service = kind.create(self.uri()).unwrap();
        Client::builder().build(service).unwrap()
    }

    pub fn server(&self) -> &MockServer {
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
        let json = fs::read_to_string(path.as_ref()).unwrap();
        let template =
            ResponseTemplate::new(status).set_body_raw(json.as_bytes(), "application/json");
        Mock::given(matcher)
            .respond_with(template)
            .mount(self.server())
            .await;
    }

    pub async fn respond<P: AsRef<Utf8Path>>(&self, status: u16, path: P) {
        self.respond_match(matchers::any(), status, path).await
    }

    pub async fn reset(&self) {
        self.server().reset().await;
    }
}
