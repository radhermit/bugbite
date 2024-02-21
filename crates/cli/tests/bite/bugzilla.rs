use std::env;

use bugbite::test::TestServer;
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;

mod attachments;
mod comments;
mod get;
mod history;
mod search;

static TEST_PATH: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TESTDATA_PATH.join("bugzilla"));

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla");
    server
}
