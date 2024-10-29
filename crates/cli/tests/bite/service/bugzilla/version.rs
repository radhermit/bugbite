use predicates::prelude::*;
use wiremock::{matchers, ResponseTemplate};

use super::*;

#[tokio::test]
async fn version() {
    let server = start_server().await;
    let template =
        ResponseTemplate::new(200).set_body_raw(r#"{"version":"5.1.1"}"#, "application/json");
    server.respond_custom(matchers::any(), template).await;

    cmd("bite bugzilla version")
        .assert()
        .stdout(predicate::str::diff("5.1.1").trim())
        .stderr("")
        .success();
}
