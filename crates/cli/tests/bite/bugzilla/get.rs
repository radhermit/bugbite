use std::{env, fs};

use predicates::prelude::*;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::command::cmd;
use crate::macros::build_path;

#[tokio::test]
async fn single_bug() {
    let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");
    let json = fs::read_to_string(path.join("bugzilla/get/single-bug.json")).unwrap();

    let mock_server = MockServer::start().await;
    let template = ResponseTemplate::new(200).set_body_raw(json.as_bytes(), "application/json");
    Mock::given(any())
        .respond_with(template)
        .mount(&mock_server)
        .await;


    let expected = fs::read_to_string(path.join("bugzilla/get/single-bug")).unwrap();
    env::set_var("BUGBITE_BASE", mock_server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");
    for subcmd in ["g", "get"] {
        cmd("bite")
            .arg(subcmd)
            .arg("12345")
            .args(&["-A", "no", "-C", "no", "-H", "no"])
            .assert()
            .stdout(predicate::str::diff(expected.clone()))
            .stderr("")
            .success();
    }
}
