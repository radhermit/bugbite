use std::env;

use bugbite::test::TestServer;
use camino::Utf8PathBuf;
use once_cell::sync::Lazy;

mod attachments;
mod comments;
mod get;
mod history;
mod search;

static TEST_DATA: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("bugbite/bugzilla"));
static TEST_OUTPUT: Lazy<Utf8PathBuf> = Lazy::new(|| crate::TEST_DATA_PATH.join("output/bugzilla"));

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla");
    server
}
