use std::env;

use bugbite::test::TestServer;

mod attachments;
mod get;
mod search;

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");
    server
}
